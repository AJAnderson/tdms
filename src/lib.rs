use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use log::debug;
use num;
pub mod tdms_datatypes;
pub use tdms_datatypes::{DataType, DataTypeRaw, DataTypeVec, TocProperties};
pub mod tdms_error;
pub use tdms_error::{TdmsError, TdmsErrorKind};

/// Key configuration flags for file reading.
#[derive(Debug)]
pub enum Endianness {
    BigEndian,
    LittleEndian,
}

#[derive(Debug)]
/// ReadPairs give the absolute file index, and the #no of bytes to read at that index, a channel is accessed by a vector of ReadPairs
pub struct ReadPair {
    start_index: u64,
    no_bytes: u64,
}

#[derive(Debug)]
pub struct ObjectMap {
    last_object: TdmsObject,
    read_map: Vec<ReadPair>,
    total_bytes: u64, 
}

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

/// A wrapper used to provide something to hang the various file read operations on
#[derive(Debug)]
pub struct FileHandle {
    handle: io::BufReader<std::fs::File>,
}

impl FileHandle {
    /// Open a Tdms file and initialize a buf rdr to handle access. Default to little endian
    /// endianess is updated when a file is read for the first time.
    pub fn open(path: &path::Path) -> Result<FileHandle, io::Error> {
        let fh = fs::File::open(path)?;
        let rdr = io::BufReader::new(fh);
        Ok(FileHandle {
            handle: rdr
        })
    }

    /// Helper function for reading u16
    pub fn match_read_u16(&mut self, endianness: &Endianness) -> Result<u16, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_u16::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_u16::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading u32 value given file endianness.
    pub fn match_read_u32(&mut self, endianness: &Endianness) -> Result<u32, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_u32::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_u32::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading u64 value given file endianness.
    pub fn match_read_u64(&mut self, endianness: &Endianness) -> Result<u64, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_u64::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_u64::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading i16
    pub fn match_read_i16(&mut self, endianness: &Endianness) -> Result<i16, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_i16::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_i16::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading i32
    pub fn match_read_i32(&mut self, endianness: &Endianness) -> Result<i32, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_i32::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_i32::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading i64
    pub fn match_read_i64(&mut self, endianness: &Endianness) -> Result<i64, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_i64::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_i64::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading f32
    pub fn match_read_f32(&mut self, endianness: &Endianness) -> Result<f32, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_f32::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_f32::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading f64
    pub fn match_read_f64(&mut self, endianness: &Endianness) -> Result<f64, io::Error> {
        let value = match endianness {
            Endianness::BigEndian => self.handle.read_f64::<BigEndian>()?,
            Endianness::LittleEndian => self.handle.read_f64::<LittleEndian>()?,
        };
        Ok(value)
    }

    /// Helper function for reading string.
    pub fn match_read_string(&mut self, endianness: &Endianness) -> Result<String, TdmsError> {
        let str_len = self.match_read_u32(endianness)?;

        let mut str_raw_buf = vec![0u8; str_len as usize];
        self.handle.read_exact(&mut str_raw_buf)?;
        Ok(String::from_utf8(str_raw_buf)?)
    }

    

    /// Reads data into the DataType enum based on the value of DataTypeRaw.
    /// The distinction exists because an enum can't have both a defined representation
    /// i.e. an integer value indicating which enum value it is, and a wrapped value
    pub fn read_datatype(&mut self, rawtype: DataTypeRaw, endianness: &Endianness) -> Result<DataType, TdmsError> {
        let dataout = match rawtype {
                DataTypeRaw::TdmsString => DataType::TdmsString(self.match_read_string(endianness)?),
                DataTypeRaw::U8 => DataType::U8(self.handle.read_u8()?),
                DataTypeRaw::U16 => DataType::U16(self.match_read_u16(endianness)?),
                DataTypeRaw::U32 => DataType::U32(self.match_read_u32(endianness)?),
                DataTypeRaw::U64 => DataType::U64(self.match_read_u64(endianness)?),
                DataTypeRaw::I8 => DataType::I8(self.handle.read_i8()?),
                DataTypeRaw::I16 => DataType::I16(self.match_read_i16(endianness)?),
                DataTypeRaw::I32 => DataType::I32(self.match_read_i32(endianness)?),
                DataTypeRaw::I64 => DataType::I64(self.match_read_i64(endianness)?),
                DataTypeRaw::SingleFloat => DataType::Float(self.match_read_f32(endianness)?),
                DataTypeRaw::DoubleFloat => DataType::Double(self.match_read_f64(endianness)?),
                DataTypeRaw::Boolean => {
                    let value = self.handle.read_u8()?;
                    let boolval: bool;
                    if value == 0 {
                        boolval = false;
                    } else {
                        boolval = true;
                    }
                    DataType::Boolean(boolval)
                }
                DataTypeRaw::TimeStamp => {
                    let epoch = self.match_read_i64(endianness)?;
                    let radix = self.match_read_u64(endianness)?;
                    DataType::TimeStamp(tdms_datatypes::TdmsTimeStamp { epoch, radix })
                }
                _ => DataType::Void(()), // TODO this is a dirty placeholder
                };          
        
        Ok(dataout)
    }

    /// Reads an array of the same type of data into a vector. It's designed to be used
    /// after a complete map of the read operations has been compiled via the map_segments function
    /// 
    /// IMPORTANT NOTE: Due to the default buffer size of BufRdr (8kb) it might not be more efficient to
    /// try and lazy load channels in the long run, as repeated seek operations at the file system level
    ///  must be performed if data is spaced more than 8kb's apart.
    ///
    /// QUESTION: Is there a better way to make a generic read operation than matching on
    /// everything all the time? It feels extremely wasteful.
    fn stub() -> bool{true}
    // TODO: Needs MAJOR work
    // #[rustfmt::skip]
    // pub fn read_data_vector(&mut self, object_map: &ObjectMap) -> Result<DataTypeVec, TdmsError> {
    //     let read_pairs = &object_map.read_map;
    //     let rawtype = object_map.last_object.raw_data_type.ok_or(TdmsError {kind: TdmsErrorKind::ObjectHasNoRawData})?;
    //     let total_bytes = object_map.total_bytes;

    //     // This only works for string initially as I really don't want to type out
    //     // all that boiler plate but don't know how to make it generic more easily
    //     let datavec: DataTypeVec = match rawtype {
    //         DataTypeRaw::TdmsString => {
    //             let mut datavec: Vec<String> = Vec::new();
    //             for pair in read_pairs {
    //                 self.handle.seek(SeekFrom::Start(pair.start_index))?;
    //                 // This is so convoluted, I already know what the data type is, why do I have
    //                 // to match it again, don't know how to fix this and keep the simplicity of
    //                 // read_datatype

    //                 //NOTE: This also does not actually handle arrays of data i.e. when ReadPair.no_bytes > 1
    //                 // It could be trivially extended with a for loop, but repeated calls to bufrdr might
    //                 // not be efficient. QUESTION: Would it be better to modify read_datatype so it can take
    //                 // an arguments for the number of reads to perform? In which case might it not become "read_data_vector"?
    //                 match self.read_datatype(rawtype, endianness)? {
    //                     DataType::TdmsString(string) => datavec.push(string),
    //                     _ => (),
    //                 };
    //             }
    //             DataTypeVec::TdmsString(datavec)
    //         }
    //         DataTypeRaw::DoubleFloat => {
    //             // TODO this could be made much more efficient I think, repeatedly allocating vectors is gauranteeed to suck.
    //             let mut datavec: Vec<f64> = Vec::with_capacity(total_bytes as usize);
                
    //             for pair in read_pairs {
    //                 println!("read len {}", pair.no_bytes);
    //                 println!("start_index: {}", pair.start_index);
    //                 self.handle.seek(SeekFrom::Start(pair.start_index))?;
    //                 for _i in 0..(pair.no_bytes / 8) {
    //                     datavec.push(self.handle.read_f64::<LittleEndian>()?);
    //                 }                    
    //             }
    //             DataTypeVec::Double(datavec) 
    //         }
    //         _ => DataTypeVec::Void(Vec::new()), // Stump implementation until I can get some feedback on generics
    //     };

    //     Ok(datavec)
    // }
}

/// Represents the contents of a Tdms file which consists of a series  of segments + ancillary data which is created to index those segments.

#[derive(Debug)]
pub struct TdmsFile {
    handle: FileHandle,
    segments: Vec<TdmsSegment>,    
    file_objects: BTreeMap<String, ObjectMap>, 
}

impl TdmsFile {
    /// Open a Tdms file and initialize a buf rdr to handle access.
    pub fn new_file(path: &path::Path) -> Result<TdmsFile, io::Error> {
        Ok(TdmsFile {
            handle: FileHandle::open(path)?,
            segments: Vec::new(),            
            file_objects: BTreeMap::new(),
        })
    }

    /// Walk the file attempting to load the segment meta data and objects.
    /// Raw data is not loaded during these reads in the interest of Lazy Loading
    /// i.e. graceful handling of very large files.
    pub fn map_segments(&mut self) -> Result<&mut Self, TdmsError> {
        // TODO: The construction of this function isn't right, if segment address ever is
        // 0xFFFF_FFFF then the file is malformed and this should probably be some kind of error.
        let mut segment_address = 0;
        while segment_address != 0xFFFF_FFFF {
            // Try read in a segment, if an error is returned, intercept it if it's
            // unexpected EoF which indicates there's nothing at the target segment
            // address, or bubble it up if it's a different kind of error.
            let segment = match TdmsSegment::new(self, segment_address) {
                Ok(segment) => segment,
                Err(err) => match &err.kind {
                    TdmsErrorKind::Io(e) => match e.kind() {
                        ErrorKind::UnexpectedEof => {
                            println!("Completed read");
                            return Ok(self);
                        }
                        // Any other io error, repackage it and send it on
                        _ => return Err(err),
                    },
                    _ => return Err(err), // Return early on weird custom errors as well
                },
            };

            // TODO I think the early return for malformed segments could happen here?
            // reverse the logical check and return if true (report an error?)
            if segment.next_seg_offset != 0xFFFF_FFFF {
                // note that next segment offset is the total number of bytes in the
                // segment minus the lead in of 28 bytes, compute the next absolute index
                segment_address = segment.next_seg_offset + segment_address + 28;
            }
            self.segments.push(segment);
        }
        Ok(self)
    }

    // Result<Vec<u64>, TdmsError>
    /// Stub implementation of load functionality, currently up to trying to get vector loading working gracefully
    // pub fn load_data(&mut self, path: &str) -> Result<DataTypeVec, TdmsError> {

    //     let object_map = self.file_objects.get(path).ok_or(TdmsError { kind: TdmsErrorKind::ChannelNotFound})?;

    //     let data = self.handle.read_data_vector(&object_map)?;

    //     Ok(data)
    // }

    /// Return a vector of channel paths
    pub fn objects(&self) -> Vec<&str> {
        let mut objects: Vec<&str> = Vec::new();

        for key in self.file_objects.keys() {
            objects.push(key)
        }
        objects
    }

    /// Diagnostic function to print current location for debugging purposes
    pub fn current_loc(&mut self) {
        println!("{:?}", self.handle.handle.seek(SeekFrom::Current(0)));
    }
}

/// A TdmsSegment consists of a 28 byte lead in followed by a series of optional MetaData properties
/// This is followed in turn by raw data if it exists.
#[derive(Debug)]
pub struct TdmsSegment {
    // Segment lead in data is 28 bytes long
    file_tag: u32, // "TDSm" always the same
    toc_mask: u32, // binary mask which generates the following flags, see tdms_datatypes.rs for reference on what each is.
    contains_metadata: bool,
    contains_rawdata: bool, 
    contains_daqmx: bool,
    interleaved: bool,
    endianness: Endianness,
    new_objects: bool,
    version_no: u32,
    next_seg_offset: u64,
    raw_data_offset: u64,
    // Then metadata and raw data (if we keep it here)
    meta_data: Option<TdmsMetaData>,
    raw_data: Option<Vec<u8>>,
    // Ancillary helper fields
    start_index: u64,
    no_chunks: u64,
}

impl fmt::Display for TdmsSegment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Segment filetag:\t{:X}", self.file_tag)?;
        writeln!(f, "Segment metadata:\t{:b}", self.toc_mask)?;
        writeln!(f, "Version no.:\t\t{}", self.version_no)?;
        writeln!(f, "Next segment offset:\t{}", self.next_seg_offset)?;
        writeln!(f, "Raw data offset:\t{}", self.raw_data_offset)?;
        writeln!(f, "No_chunks:\t{}", self.no_chunks)?;

        Ok(())
    }
}

impl TdmsSegment {

    /// construct a TDMS segment, indirect as necessary
    fn new(file: &mut TdmsFile, index: u64) -> Result<TdmsSegment, TdmsError> {
        Ok(TdmsSegment::_new(index)?._read(file)?)
    }

    fn _new(index: u64) -> Result<TdmsSegment, TdmsError> {
        Ok(TdmsSegment {
            start_index: index,
            file_tag: 0,
            toc_mask: 0,
            contains_metadata: false,
            contains_rawdata: false, 
            contains_daqmx: false,
            interleaved: false,
            endianness: Endianness::LittleEndian,
            new_objects: false,
            version_no: 0,
            next_seg_offset: 0,
            raw_data_offset: 0,
            meta_data: None,
            raw_data: None,
            no_chunks: 0,
        })
    }

    /// Load in a segment and parse all objects and properties, does not load raw data. This allows lazy loading to handle very large files.
    fn _read(mut self, file: &mut TdmsFile) -> Result<TdmsSegment, TdmsError> {
        // Seek to the "absolute index" (relative to start) This index has to be built up for each segment as we go. This is handled in the map_segments function
        let target_loc = file.handle.handle.seek(SeekFrom::Start(self.start_index))?;
        debug!("Target Loc: {}", target_loc);
        
        self.read_lead_in(file)?;

        let current_loc = file.handle.handle.seek(SeekFrom::Current(0))?; // position at end of lead in read
        debug!("current_loc: {}", current_loc);

        // Load the meta_data for this segment TODO 2) does there need to be a check of kToCNewContents?
        let meta_data = TdmsMetaData::new(file, &self.endianness)?;
        debug!("chunk size: {}", meta_data.chunk_size);
        let no_chunks = (self.next_seg_offset - self.raw_data_offset) / meta_data.chunk_size;
        debug!("no_chunks: {}", no_chunks);

        // Add or update objects in the file_objects map for interleaved data

        // for object in meta_data.objects.iter() {
        //     if let Some(raw_data_size) = object.total_size {
                
        //     }
        // }


        // Add or update objects in the file_objects map for non-interleaved data
        let mut relative_position: u64 = 0;
        for object in meta_data.objects.iter() {
            // If the object has data in this segment create a read map for that data
            if let Some(raw_data_size) = object.total_size {
                let mut new_read_map: Vec<ReadPair> = Vec::new();
                for i in 0..no_chunks {
                    let pair = ReadPair {
                        start_index: self.start_index + 28 + self.raw_data_offset + i * meta_data.chunk_size + relative_position,
                        no_bytes: raw_data_size,
                    };
                    new_read_map.push(pair);
                }

                // Check if the object exists in the high level map, if so update it
                // otherwise insert the new set of read pairs
                file.file_objects
                    .entry(object.object_path.clone())
                    .and_modify(|object_map| 
                        { object_map.last_object = object.clone(); 
                        object_map.read_map.append(&mut new_read_map);
                        object_map.total_bytes = object_map.total_bytes + raw_data_size;})
                    .or_insert(ObjectMap {last_object: object.clone(), read_map: new_read_map, total_bytes: raw_data_size});

                relative_position = relative_position + raw_data_size;
            }
        }
        // Return the initialised Segment with lead in and metadata
        Ok(self)
    }

    fn read_lead_in(&mut self, file: &mut TdmsFile) -> Result<&mut TdmsSegment, TdmsError> {
        // Convert the critical lead in information to appropriate representation
        let file_tag: u32 = file.handle.match_read_u32(&self.endianness)?;
        let toc_mask: u32 = file.handle.match_read_u32(&self.endianness)?;

        debug!("File tag: {}", file_tag);
        debug!("toc_mask: {:b}", toc_mask);

        if (toc_mask & TocProperties::KTocMetaData as u32) != 0 {
            self.contains_metadata = true        
        }

        if (toc_mask & TocProperties::KTocRawData as u32) != 0 {
            self.contains_rawdata = true        
        }

        if (toc_mask & TocProperties::KTocDAQmxRawData as u32) != 0 {
            self.contains_daqmx = true
        }

        if (toc_mask & TocProperties::KTocInterleavedData as u32) != 0 {
            self.interleaved = true
        }

        if (toc_mask & TocProperties::KTocBigEndian as u32) != 0 {
            self.endianness = Endianness::BigEndian
        };

        if (toc_mask & TocProperties::KTocNewObjList as u32) != 0 {
            self.new_objects = true
        }

        debug!("Endianess {:?}", self.endianness);

        // Finish out the lead in based on whether the data is little endian
        self.version_no = file.handle.match_read_u32(&self.endianness)?;
        self.next_seg_offset = file.handle.match_read_u64(&self.endianness)?;
        self.raw_data_offset = file.handle.match_read_u64(&self.endianness)?;
        debug!("version_no: {}", self.version_no);
        debug!("next_seg_offset: {}", self.next_seg_offset);
        debug!("raw_data_offset: {}", self.raw_data_offset);

        Ok(self)
    }
}

#[derive(Debug)]
pub struct TdmsMetaData {
    no_objects: u32,
    objects: Vec<TdmsObject>,
    // chunk_size is used in combination with segment index information to
    // figure out how many blocks of channel data there are in any given
    // segment
    chunk_size: u64,
}

impl fmt::Display for TdmsMetaData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "No. objects:\t{}", self.no_objects)?;
        writeln!(f, "Chunk Size:\t{}", self.chunk_size)?;
        for obj in &self.objects {
            writeln!(f, "__Object__")?;
            write!(f, "{}", obj)?;
        }

        Ok(())
    }
}

impl TdmsMetaData {
    /// Creates a new meta data struct and reads objects into it.
    /// abs_data_index points to the index of raw data in the segment
    /// with respect to the start of the file.
    pub fn new(file: &mut TdmsFile, endianness: &Endianness) -> Result<TdmsMetaData, TdmsError> {
        Ok(TdmsMetaData::_new(&mut file.handle, endianness)?._read_meta_data(file, endianness)?)
    }

    fn _new(file_handle: &mut FileHandle, endianness: &Endianness) -> Result<TdmsMetaData, TdmsError> {
        let no_objects = file_handle.match_read_u32(endianness)?;
        debug!("no_objects: {}", no_objects);
        Ok(TdmsMetaData {
            no_objects,
            objects: Vec::new(),
            chunk_size: 0,
        })
    }

    /// Read in objects, keep track of accumlating channel size so objects can be loaded
    /// later by directly addressing their constituent addresses
    #[rustfmt::skip]
    fn _read_meta_data(mut self, file: &mut TdmsFile, endianness: &Endianness) -> Result<TdmsMetaData, TdmsError> {
        let mut chunk_size: u64 = 0;

        for i in 0..self.no_objects {
            debug!("-----------------------------------");
            debug!("object #: {}", i);

            // Read in an object including properties
            let obj = TdmsObject::read_object(file, endianness)?;
            if let Some(size) = obj.total_size {
                chunk_size = chunk_size + size
            }
            
            self.objects.push(obj);
        }
        // Store the chunk size for raw data in the segment.
        self.chunk_size = chunk_size;
        Ok(self)
    }
}

/// TODO: Implement this so that we don't copy the properties vector when we only care about  the raw data subset.
// pub struct TdmsObjectRawData {

// }

#[derive(Debug, Clone)]
pub struct TdmsObject {
    object_path: String,
    index_info_len: u32, // The length in bytes of the indexing info for raw data, including the length of this field. Should always be 20 (defined lenght) or 28 (variable length)
    raw_data_type: Option<DataTypeRaw>, // appears in file as u32.
    raw_data_dim: Option<u32>,
    no_raw_vals: Option<u64>,
    total_size: Option<u64>, // in bytes, appears in file for variable length types only.
    no_properties: u32,
    properties: Option<Vec<ObjectProperty>>,
}

impl fmt::Display for TdmsObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // writeln!(f, "Obj path len:\t{}", self.object_path_len)?;
        writeln!(f, "Obj path:\t{}", self.object_path)?;
        writeln!(f, "Index info length:\t{:x}", self.index_info_len)?;
        writeln!(f, "Raw data type:\t{:?}", self.raw_data_type)?;
        writeln!(f, "Raw data dim:\t{:?}", self.raw_data_dim)?;
        writeln!(f, "No. raw vals:\t{:?}", self.no_raw_vals)?;
        writeln!(f, "Total size:\t{:?}", self.total_size)?;
        writeln!(f, "No. properties:\t{:?}", self.no_properties)?;
        match &self.properties {
            Some(props) => {
                for property in props {
                    writeln!(f, "__Property__")?;
                    write!(f, "{}", property)?;
                }
            }
            None => (),
        }

        Ok(())
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
        // writeln!(f, "Property name len: {}", self.prop_name_len)?;
        writeln!(f, "Property name: {}", self.prop_name)?;
        writeln!(f, "Property datatype: {:?}", self.data_type)?;
        writeln!(f, "Property val: {:?}", self.property)?;
        Ok(())
    }
}

impl TdmsObject {
    /// Read an object from file including its properties    
    pub fn read_object(file: &mut TdmsFile, endianness: &Endianness) -> Result<TdmsObject, TdmsError> {
        let path = file.handle.match_read_string(endianness)?;
        debug!("obj_path: {}", path);        

        // Pre-initialize object
        let mut new_object = TdmsObject {
            object_path: path,
            index_info_len: 0,
            raw_data_type: None,
            raw_data_dim: None,
            no_raw_vals: None,
            total_size: None,
            no_properties: 0,
            properties: None,
        };

        new_object.index_info_len = file.handle.match_read_u32(&endianness)?;
        debug!("- data_index_len:  {:?}", new_object.index_info_len);

        // TODO: Need to handle DAQmx data types here.
        if new_object.index_info_len == 0xFFFF_FFFF {
            // No raw data in this object, do nothing
        } else if new_object.index_info_len == 0 {
            // raw data for this object is identical to previous segments, copy the raw data across TODO: It's not enough to just grab last index, we will have to drill into some kind of object map.
            // let previous_object = file
            //     .segments
            //     .last()
            //     .ok_or(TdmsError {
            //         kind: TdmsErrorKind::NoPreviousSegment,
            //     })?
            //     .meta_data
            //     .as_ref()
            //     .ok_or(TdmsError {
            //         kind: TdmsErrorKind::NoMetaDataAvailable,
            //     })?
            //     .objects
            //     .get(&new_object.object_path)
            //     .ok_or(TdmsError {
            //         kind: TdmsErrorKind::NoPreviousObject,
            //     })?;
            let object_map = file.file_objects.get(&new_object.object_path).ok_or(TdmsError { kind: TdmsErrorKind::NoPreviousObject,})?;
            let previous_object = &object_map.last_object;

            new_object.raw_data_type = previous_object.raw_data_type;
            new_object.raw_data_dim = previous_object.raw_data_dim;
            new_object.no_raw_vals = previous_object.no_raw_vals;
            new_object.total_size = previous_object.total_size;
            new_object.index_info_len = previous_object.index_info_len;
        } else {
            // This is a fresh, non DAQmx object.
            // read a u32 and attempt to convert it to a DataTypeRaw enum value. Propagate an error if this fails.
            let raw_data_type =
                num::FromPrimitive::from_u32(file.handle.match_read_u32(&endianness)?).ok_or(TdmsError {
                    kind: TdmsErrorKind::RawDataTypeNotFound,
                })?;
            // stage the read so we can use the information to compute total size
            let dim = file.handle.match_read_u32(&endianness)?;
            let no_vals = file.handle.match_read_u64(&endianness)?;

            // total_size is either recorded in the file if data is TdmsString or else
            // must be computed. Size can return an error if called on DataTypeRaw::TdmsString which is why there is a guard clause here.
            let total_size = match raw_data_type {
                DataTypeRaw::TdmsString => file.handle.match_read_u64(&endianness)?,
                other => other.size()? * no_vals * dim as u64,
            };
            new_object.raw_data_type = Some(raw_data_type);
            new_object.raw_data_dim = Some(dim);
            new_object.no_raw_vals = Some(no_vals);
            new_object.total_size = Some(total_size);
        };
        debug!("- data_type:  {:?}", new_object.raw_data_type);
        debug!("- data_dim:  {:?}", new_object.raw_data_dim);
        debug!("- no_vals:  {:?}", new_object.no_raw_vals);
        debug!("- total_size:  {:?}", new_object.total_size);

        // Read the object properties
        new_object.no_properties = file.handle.match_read_u32(&endianness)?;
        debug!("== no_properties:  {:?}", new_object.no_properties);
        if new_object.no_properties > 0 {
            let mut temp_vec = Vec::new();
            for _i in 0..new_object.no_properties {
                temp_vec.push(ObjectProperty::read_property(&mut file.handle, &endianness)?);
            }
            new_object.properties = Some(temp_vec);
        }

        Ok(new_object)
    }
}

impl ObjectProperty {
    /// Read properties associated with an object
    pub fn read_property(file: &mut FileHandle, endianness: &Endianness) -> Result<ObjectProperty, TdmsError> {
        let prop_name = file.match_read_string(endianness)?;
        debug!("prop_name: {}", prop_name);
        // Read in a u32 and attempt to convert to a variant of DataTypeRaw. Raise an error if this fails.
        let prop_datatype =
            num::FromPrimitive::from_u32(file.match_read_u32(endianness)?).ok_or(TdmsError {
                kind: TdmsErrorKind::RawDataTypeNotFound,
            })?;
        // debug!("prop_datatype {:?}", prop_datatype);
        let property = file.read_datatype(prop_datatype, endianness)?;
        debug!("property: {:?}", property);

        Ok(ObjectProperty {
            prop_name,
            data_type: prop_datatype,
            property,
        })
    }
}
