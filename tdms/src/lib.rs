use indexmap::IndexMap;
use std::fmt;
use std::fs;
use std::io;
use std::io::{BufReader, ErrorKind, Read, Seek, SeekFrom};
use std::path;

use byteorder::{BE, LE, *};
use log::debug;
pub mod tdms_datatypes;
pub use tdms_datatypes::DataTypeVec;
use tdms_datatypes::{
    read_data_vector, read_datatype, read_string, DataType, DataTypeRaw, TocMask, TocProperties,
};
pub mod tdms_error;
pub use tdms_error::{Result, TdmsError};

const HEADER_LEN: u64 = 28;
const NO_RAW_DATA: u32 = 0xFFFF_FFFF;
const DATA_INDEX_MATCHES_PREVIOUS: u32 = 0x0000_000;
const FORMAT_CHANGING_SCALER: u32 = 0x6912_0000;
const DIGITAL_LINE_SCALER: u32 = 0x6912_0000;
/*
The TDMS file structure consists of a series of segments which contain metadata regarding the file.
Each segment contains any number of group objects, each of which can contain any number of properties.
Segment
-Objects
--Properties

The object hierarchy is always(?) encoded as
Root Object
-Group Object
--Channel Object
*/

#[derive(Debug, Clone)]
/// ReadPairs give the absolute file index, and the #no of bytes to read at that index, a channel
/// is accessed by a vector of ReadPairs, the length of which should correspond to the number of
/// raw data chunks in the file in which the channel is present.
pub struct ReadPair {
    start_index: u64,
    no_values: u64,
    interleaved: bool,
    /// This is the sum of the datatype sizes for all channels in the chunk i.e. the number of bytes till
    /// the next value of this channel in interleaved data. Only present if interleaved is true.
    stride: Option<u64>,
}

impl fmt::Display for ReadPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "start: {}\t no_values: {}",
            self.start_index, self.no_values
        )?;
        Ok(())
    }
}

/// A struct to maintain the vector of read pairs associated with a particular object (channel), as well as keep track of the object and any properties it accrues throughout the reading process. The set of maps for each object are maintained within the main "TdmsMap" struct via a hash map.
#[derive(Debug, Clone, Default)]
pub struct ObjectMap {
    last_object: TdmsObject, // the most up to date version of the object, properties and indexing information are copied to this.
    read_map: Vec<ReadPair>, // for each segment in the file a vector of read pairs exist.
    total_bytes: u64, // The total byte count of raw data associated with the object, for keeping track of locations in file
    total_values: usize, // Used to allocate memory to read the data
    bigendian: bool,  // whether the object associated with this map has been logged as bigendian
}

impl fmt::Display for ObjectMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Object:\t{}", self.last_object)?;

        Ok(())
    }
}

//handle: io::BufReader<std::fs::File>,

pub struct TdmsFile {
    reader: BufReader<fs::File>,
    tdms_map: TdmsMap,
}

impl TdmsFile {
    /// Open a Tdms file and initialize a buf rdr to handle access.
    pub fn open(path: &path::Path) -> Result<TdmsFile> {
        let fh = fs::File::open(path)?;
        let file_length = fh.metadata().unwrap().len();
        println!("file size on load: {:?}", file_length);
        let mut reader = io::BufReader::new(fh);
        let mut tdms_map = TdmsMap::new();
        tdms_map.map_segments(&mut reader, file_length)?;

        Ok(TdmsFile { reader, tdms_map })
    }

    /// Stub implementation of load functionality, currently up to trying to get vector loading working gracefully
    pub fn load_data(&mut self, path: &str) -> Result<DataTypeVec> {
        // check if object exists in map

        let object_map = self
            .tdms_map
            .all_objects
            .get(path)
            .ok_or(TdmsError::ChannelNotFound)?;
        if object_map.bigendian {
            Ok(read_data_vector::<_, BE>(object_map, &mut self.reader)?)
        } else {
            Ok(read_data_vector::<_, LE>(object_map, &mut self.reader)?)
        }
    }

    /// Return a vector of object paths
    pub fn all_objects(&self) -> Vec<&str> {
        let mut objects: Vec<&str> = Vec::new();

        for key in self.tdms_map.all_objects.keys() {
            objects.push(key)
        }
        objects
    }

    /// Return a vector of channel paths for channels with data
    pub fn data_objects(&self) -> Vec<&str> {
        let mut objects: Vec<&str> = Vec::new();

        for (key, object_map) in &self.tdms_map.all_objects {
            if object_map.last_object.no_bytes > 0 {
                objects.push(key);
            }
        }
        objects
    }

    /// Display an objects properties
    pub fn object_properties(&self, path: &str) -> Result<&TdmsObject> {
        let object = self
            .tdms_map
            .all_objects
            .get(path)
            .ok_or(TdmsError::ChannelNotFound)?;

        Ok(&object.last_object)
    }

    /// Print an object's read pairs
    pub fn object_with_read_pairs(&self, path: &str) -> Result<()> {
        let object = self
            .tdms_map
            .all_objects
            .get(path)
            .ok_or(TdmsError::ChannelNotFound)?;

        print!("{:?}", object);
        Ok(())
    }
}

/// Diagnostic function to print current location for debugging purposes
pub fn current_loc<R: Read + Seek>(reader: &mut R) {
    println!("{:?}", reader.seek(SeekFrom::Current(0)));
}

/// Represents the contents of a Tdms file which consists of a series  of segments + ancillary data which is created to index those segments.
#[derive(Debug)]
pub struct TdmsMap {
    segments: Vec<TdmsSegment>,
    // Keeps track of all objects in file and their read maps
    pub all_objects: IndexMap<String, ObjectMap>, 
    // Keeps track of order of objects accumulated over segments, is reset when kToCNewObjectList flag is detected
    live_objects: Vec<String>, 
}

impl TdmsMap {
    fn new() -> TdmsMap {
        TdmsMap {
            segments: Vec::new(),
            all_objects: IndexMap::new(),
            live_objects: Vec::new(),
        }
    }

    /// Walk the file attempting to load the segment meta data and objects.
    /// Raw data is not loaded during these reads in the interest of Lazy Loading
    /// i.e. memory efficienct handling of very large files.
    fn map_segments<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        file_length: u64,
    ) -> Result<&mut Self> {
        let mut next_segment_address = 0;

        // If the file is corrupted, the last segment will contain 0xFFFF_FFFF for the "next segment offset".
        // In this case the reader will attempt to map the segment but will hit an Unexpected end of file error
        // while doing so.
        while next_segment_address < file_length {
            // Try read in a segment, if an error is returned, intercept it if it's
            // unexpected EoF which indicates there's nothing at the target segment
            // address, or bubble it up if it's a different kind of error.

            let segment = match self.read_segment(reader, next_segment_address) {
                Ok(segment) => segment,
                Err(err) => match &err {
                    TdmsError::Io(e) => match e.kind() {
                        ErrorKind::UnexpectedEof => {
                            println!("Completed read, final segment is corrupted");
                            return Ok(self);
                        }
                        // Any other io error, repackage it and send it on
                        _ => return Err(err),
                    },
                    _ => return Err(err), // Return early on weird custom errors as well
                },
            };

            next_segment_address = segment.next_seg_offset + next_segment_address + HEADER_LEN;

            self.segments.push(segment);
        }
        println!("Completed read");
        Ok(self)
    }

    /// Load in a segment and parse all objects and properties, does not load raw data.
    /// This allows lazy loading to handle very large files.
    fn read_segment<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        start_index: u64,
    ) -> Result<TdmsSegment> {
        // Seek to the "absolute index" (relative to start) This index has to be built up for each segment as we go.
        // This is handled in the map_segments function
        reader.seek(SeekFrom::Start(start_index))?;

        let mut segment = TdmsSegment::new(start_index);

        // Convert the critical lead in information to appropriate representation, we know the
        // first part of the lead in is little endian so we save a check here.
        segment.file_tag = reader.read_u32::<LE>()?;
        segment.toc_mask = TocMask::from_flags(reader.read_u32::<LE>()?);

        if segment.toc_mask.has_flag(TocProperties::KTocBigEndian) {
            self.read_segment_metadata::<R, BE>(reader, segment)
        } else {
            self.read_segment_metadata::<R, LE>(reader, segment)
        }
    }

    fn read_segment_metadata<R: Read + Seek, O: ByteOrder>(
        &mut self,
        reader: &mut R,
        mut segment: TdmsSegment,
    ) -> Result<TdmsSegment> {
        debug!("_______ENTERING SEGMENT________");
        // Finish out the lead in
        segment.version_no = reader.read_u32::<O>()?;
        segment.next_seg_offset = reader.read_u64::<O>()?;
        segment.raw_data_offset = reader.read_u64::<O>()?;

        debug!(
            "NewObjFlag?: {}",
            segment.toc_mask.has_flag(TocProperties::KTocNewObjList)
        );

        // Load the meta_data for this segment, parsing objects that appear in this segment
        let no_objects = reader.read_u32::<O>()?;

        let mut chunk_size: u64 = 0;
        let mut channels_size: u64 = 0;

        // Add any previous index size info if objects are simply new additions
        if !segment.toc_mask.has_flag(TocProperties::KTocNewObjList) {
            chunk_size = self.segments.last().unwrap().chunk_size;
            channels_size = self.segments.last().unwrap().chunk_size;            
        } else {
            // If it's all new objects or a reshuffled list, clear the old list
            self.live_objects.clear();
        }

        for _i in 0..no_objects {
            let path = read_string::<R, O>(reader)?;
            // Read in an object including properties
            let (no_bytes, raw_data_type) = TdmsObject::update_read_object::<R, O>(self, path.clone(), reader)?;            
            
            // Keep track of the accumulating raw data size for objects
            chunk_size += no_bytes;
            if let Some(raw_type) = raw_data_type {
                channels_size += match raw_type {
                    DataTypeRaw::TdmsString => no_bytes, // TODO no idea if this is correct i.e. how strings interleave
                    other => other.size()?,
                };
            };

            // If the object isn't in the live objects then it is truly new, so push it. If it is there
            // then something about the object has changed but its order is still correct.
            if !self.live_objects.contains(&path) {
                self.live_objects.push(path.clone());
            }            
        }
        
        segment.no_chunks = if chunk_size > 0 {
            (segment.next_seg_offset - segment.raw_data_offset) / chunk_size
        } else {
            0
        };

        self.update_indexes(&segment, chunk_size, channels_size)?;
        Ok(segment)
    }    

    fn update_indexes(&mut self, segment: &TdmsSegment, chunk_size: u64, channels_size: u64) -> Result<()> {
        let mut relative_position: u64 = 0; // Used in computing read pairs as we go
        for key in self.live_objects.iter() {
            let object_map = self.all_objects.get_mut(key).unwrap();
            let type_size = if let Some(raw_type) = object_map.last_object.raw_data_type {
                match raw_type {
                    // TODO no idea if this is correct i.e. how strings interleave
                    DataTypeRaw::TdmsString => object_map.last_object.no_bytes,
                    other => other.size()?,
                }
            } else {
                0
            };
            debug!("Type Size: {}", type_size);

            //compute read pairs as we go to save double iteration over the objects map,
            // only compute if size here is > 0
            if object_map.last_object.no_bytes > 0 {
                for i in 0..segment.no_chunks {
                    let pair = ReadPair {
                        start_index: segment.start_index
                            + HEADER_LEN
                            + segment.raw_data_offset
                            + i * chunk_size
                            + relative_position,
                        no_values: object_map.last_object.no_raw_vals.unwrap(),
                        interleaved: segment
                            .toc_mask
                            .has_flag(TocProperties::KTocInterleavedData),
                        stride: Some(channels_size - type_size),
                    };

                    debug!("Read Pair {:?}", pair);

                    object_map.read_map.push(pair);
                    object_map.total_bytes += object_map.last_object.no_bytes;
                    object_map.total_values += object_map.last_object.no_raw_vals.unwrap() as usize;
                    debug!("Accum values: {}", object_map.total_values);
                }
            };

            debug!("Accum Obj Size: {}", object_map.total_bytes);

            object_map.bigendian = segment.toc_mask.has_flag(TocProperties::KTocBigEndian);

            // If interleaved then the start position depends on the item sizes, if continuous
            // then it's the number of values x type size i.e. "total_bytes"
            debug!(
                "Interleaved data: {}",
                segment
                    .toc_mask
                    .has_flag(TocProperties::KTocInterleavedData)
            );
            debug!("Flags: {:b}", segment.toc_mask.flags);
            if segment
                .toc_mask
                .has_flag(TocProperties::KTocInterleavedData)
            {
                relative_position += type_size;
            } else {
                relative_position += object_map.last_object.no_bytes;
            }
            debug!("relative position: {}", relative_position);
        }
        Ok(())
    }
}

/// A TdmsSegment consists of a 28 byte lead in followed by a series of optional MetaData
/// properties. This is followed in turn by raw data if it exists.
#[derive(Debug)]
pub struct TdmsSegment {
    // Segment lead in data is 28 bytes long
    file_tag: u32, // "TDSm" always the same
    toc_mask: TocMask,
    version_no: u32,
    next_seg_offset: u64,
    raw_data_offset: u64,
    // Ancillary helper fields
    start_index: u64,
    no_chunks: u64,
    // chunk_size is used in combination with segment index information to
    // figure out how many blocks of channel data there are in any given
    // segment
    chunk_size: u64,
    /// The sum total of byte sizes for each channel's data type
    channels_size: u64,
}

impl fmt::Display for TdmsSegment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Segment filetag:\t{:X}", self.file_tag)?;
        writeln!(f, "Segment flags:\t{:?}", self.toc_mask)?;
        writeln!(f, "Version no.:\t\t{}", self.version_no)?;
        writeln!(f, "Next segment offset:\t{}", self.next_seg_offset)?;
        writeln!(f, "Raw data offset:\t{}", self.raw_data_offset)?;
        writeln!(f, "No_chunks:\t{}", self.no_chunks)?;
        Ok(())
    }
}

impl TdmsSegment {
    pub fn new(start_index: u64) -> TdmsSegment {
        TdmsSegment {
            start_index,
            file_tag: 0,
            toc_mask: TocMask::from_flags(0),
            version_no: 0,
            next_seg_offset: 0,
            raw_data_offset: 0,
            no_chunks: 0,
            chunk_size: 0,
            channels_size: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TdmsObject {
    object_path: String,
    index_info_len: u32, // The length in bytes of the indexing info for raw data, including the length of this field. Should always be 20 (defined length) or 28 (variable length)
    raw_data_type: Option<DataTypeRaw>, // appears in file as u32.
    raw_data_dim: Option<u32>,
    no_raw_vals: Option<u64>,
    no_bytes: u64, // of raw data in bytes, appears in file for variable length types (String) only. comptued otherwise
    no_properties: u32,
    daqmx_info: Option<DAQMxInfo>,
    properties: IndexMap<String, ObjectProperty>,
}

#[derive(Debug, Clone)]
pub struct DAQMxInfo {
    formatvec_size: u32,
    scalers: Vec<DAQMxScaler>,
    widthvec_size: u32,
    widthvec: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct DAQMxScaler {
    daqmx_data_type: DataTypeRaw,
    daqmx_rawbuff_indx: u32,
    daqmx_raw_byte_offset: u32,
    sample_format_bitmap: u32,
    scale_id: u32,
}

impl DAQMxScaler {
    pub fn new<R: Read + Seek, O: ByteOrder>(reader: &mut R) -> Result<DAQMxScaler> {
        let scaler = DAQMxScaler {
            daqmx_data_type: DataTypeRaw::from_u32(reader.read_u32::<O>()?)?,
            daqmx_rawbuff_indx: reader.read_u32::<O>()?,
            daqmx_raw_byte_offset: reader.read_u32::<O>()?,
            sample_format_bitmap: reader.read_u32::<O>()?,
            scale_id: reader.read_u32::<O>()?,
        };
        Ok(scaler)
    }
}

impl fmt::Display for TdmsObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Obj path:\t{}", self.object_path)?;
        writeln!(f, "Index info length:\t{:x}", self.index_info_len)?;
        writeln!(f, "Raw data type:\t{:?}", self.raw_data_type)?;
        writeln!(f, "Raw data dim:\t{:?}", self.raw_data_dim)?;
        writeln!(f, "No. raw vals:\t{:?}", self.no_raw_vals)?;
        writeln!(f, "Total size:\t{:?}", self.no_bytes)?;
        writeln!(f, "No. properties:\t{:?}", self.no_properties)?;
        writeln!(f, "Actual property count:\t{:?}", self.properties.len())?;
        for (_key, property) in self.properties.iter() {
            writeln!(f, "__Property__")?;
            write!(f, "{}", property)?;
        }

        Ok(())
    }
}

impl TdmsObject {
    /// Read an object from file including its properties, update the object's information
    /// in the all_objects map.
    pub fn update_read_object<R: Read + Seek, O: ByteOrder>(
        tdms_map: &mut TdmsMap,
        path: String,
        reader: &mut R,
    ) -> Result<(u64, Option<DataTypeRaw>)> {
        // check existence now for later use
        let prior_object = tdms_map.all_objects.contains_key(&path);

        // Try to obtain a reference to the last record of the objects
        // to update in place, create a default entry if none present
        let new_object = &mut tdms_map
            .all_objects
            .entry(path.clone())
            .or_default()
            .last_object;

        debug!("object_path: {}", path);
        new_object.object_path = path;
        for live in &tdms_map.live_objects {
            debug!("Map object: {}", live);
        }

        new_object.index_info_len = reader.read_u32::<O>()?;

        debug!("index len: {}", new_object.index_info_len);
        if new_object.index_info_len == NO_RAW_DATA {
            new_object.update_properties::<R, O>(reader)?;
        } else if new_object.index_info_len == DATA_INDEX_MATCHES_PREVIOUS {
            // raw data index for this object should be identical to previous segments.
            if !prior_object {
                return Err(TdmsError::NoPreviousObject);
            } else {
                new_object.update_properties::<R, O>(reader)?;
            }
        } else if new_object.index_info_len == FORMAT_CHANGING_SCALER {
            new_object.read_sizeinfo::<R, O>(reader)?;
            new_object.read_daqmxinfo::<R, O>(reader)?;
            new_object.update_properties::<R, O>(reader)?;
        } else if new_object.index_info_len == DIGITAL_LINE_SCALER {
            new_object.read_sizeinfo::<R, O>(reader)?;
            new_object.read_daqmxinfo::<R, O>(reader)?;
            new_object.update_properties::<R, O>(reader)?;
        } else {
            // This is a fresh, non DAQmx object, or amount of data has changed
            new_object.read_sizeinfo::<R, O>(reader)?;
            new_object.update_properties::<R, O>(reader)?;
        }
        Ok((new_object.no_bytes, new_object.raw_data_type))
    }

    fn read_sizeinfo<R: Read + Seek, O: ByteOrder>(&mut self, reader: &mut R) -> Result<&mut Self> {
        let raw_data_type = DataTypeRaw::from_u32(reader.read_u32::<O>()?)?;
        let dim = reader.read_u32::<O>()?;
        let no_vals = reader.read_u64::<O>()?;

        // total_bytes (bytes) is either recorded in the file if data is TdmsString or else
        // must be computed. Size() will return an error if called on DataTypeRaw::TdmsString
        // which is why there is a guard clause here.
        self.no_bytes = match raw_data_type {
            DataTypeRaw::TdmsString => reader.read_u64::<O>()?,
            other => other.size()? * no_vals * dim as u64,
        };
        debug!("Object total bytes: {}", self.no_bytes);
        debug!("Data Dim: {}", dim);
        debug!("No Raw Vals: {}", no_vals);
        self.raw_data_type = Some(raw_data_type);
        self.raw_data_dim = Some(dim);
        self.no_raw_vals = Some(no_vals);

        Ok(self)
    }

    fn read_daqmxinfo<R: Read + Seek, O: ByteOrder>(
        &mut self,
        reader: &mut R,
    ) -> Result<&mut Self> {
        let daqmx_formatvec_size = reader.read_u32::<O>()?;

        let mut scalers: Vec<DAQMxScaler> = Vec::new();
        for _i in 0..daqmx_formatvec_size {
            let scaler = DAQMxScaler::new::<R, O>(reader)?;
            scalers.push(scaler);
        }

        let daqmx_datawidthvec_size = reader.read_u32::<O>()?;
        let mut daqmx_data_width_vec = Vec::with_capacity(daqmx_datawidthvec_size as usize);
        for _i in 0..daqmx_datawidthvec_size {
            daqmx_data_width_vec.push(reader.read_u32::<O>()?);
        }

        self.daqmx_info = Some(DAQMxInfo {
            formatvec_size: daqmx_formatvec_size,
            scalers,
            widthvec_size: daqmx_datawidthvec_size,
            widthvec: daqmx_data_width_vec,
        });

        Ok(self)
    }

    /// Read the object properties, update if that property already exists for that object
    fn update_properties<R: Read + Seek, O: ByteOrder>(
        &mut self,
        reader: &mut R,
    ) -> Result<&mut Self> {
        self.no_properties = reader.read_u32::<O>()?;
        if self.no_properties > 0 {
            for _i in 0..self.no_properties {
                let property = ObjectProperty::read_property::<R, O>(reader)?;
                // overwrite the previous version of the property or else insert new property
                self.properties.insert(property.prop_name.clone(), property);
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Clone)]
pub struct ObjectProperty {
    prop_name: String,
    data_type: DataTypeRaw,
    property: DataType,
}

impl fmt::Display for ObjectProperty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Property name: {}", self.prop_name)?;
        writeln!(f, "Property datatype: {:?}", self.data_type)?;
        writeln!(f, "Property val: {:?}", self.property)?;
        Ok(())
    }
}

impl ObjectProperty {
    /// Instantiate a property and read into it.
    pub fn read_property<R: Read + Seek, O: ByteOrder>(reader: &mut R) -> Result<ObjectProperty> {
        let prop_name = read_string::<R, O>(reader)?;
        let data_type = DataTypeRaw::from_u32(reader.read_u32::<O>()?)?;
        let property = read_datatype::<R, O>(reader, data_type)?;
        Ok(ObjectProperty {
            prop_name,
            data_type,
            property,
        })
    }
}
