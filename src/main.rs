use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use num;
mod tdms_datatypes;
use tdms_datatypes::{DataType, DataTypeRaw, DataTypeVec, TocProperties};

/// Custom error type to handle multiple types of errors
/// Needs: Variant for parse errors
#[derive(Debug)]
pub enum TdmsError {
    Io(io::Error),
    Custom(String),
}

// QUESTION: How to I convert this to &str? Do I want to?
impl From<String> for TdmsError {
    fn from(msg: String) -> TdmsError {
        TdmsError::Custom(msg)
    }
}

impl From<std::io::Error> for TdmsError {
    fn from(err: std::io::Error) -> TdmsError {
        TdmsError::Io(err)
    }
}

#[derive(Debug)]
pub enum Endianess {
    BigEndian,
    LittleEndian,
}

#[derive(Debug)]
pub enum Interleaved {
    Interleaved,
    Regular,
}

#[derive(Debug)]
pub struct ReadPair {
    start_index: u64,
    no_elements: u64,
}

/// Helper function for reading u32 value given file endianess. Useful enough to be promoted to first order
pub fn match_read_u32(file: &mut TdmsFileHandle) -> Result<u32, io::Error> {
    let value = match file.endianess {
        Endianess::BigEndian => file.handle.read_u32::<BigEndian>()?,
        Endianess::LittleEndian => file.handle.read_u32::<LittleEndian>()?,
    };
    Ok(value)
}

/// Helper function for reading u64 value given file endianess. Useful enough to be promoted to first order
pub fn match_read_u64(file: &mut TdmsFileHandle) -> Result<u64, io::Error> {
    let value = match file.endianess {
        Endianess::BigEndian => file.handle.read_u64::<BigEndian>()?,
        Endianess::LittleEndian => file.handle.read_u64::<LittleEndian>()?,
    };
    Ok(value)
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
pub struct TdmsFileHandle {
    handle: io::BufReader<std::fs::File>,
    endianess: Endianess,
}

impl TdmsFileHandle {
    /// Open a Tdms file and initialize a buf rdr to handle access. Default to little endian
    pub fn open(path: &path::Path) -> Result<TdmsFileHandle, io::Error> {
        let fh = fs::File::open(path)?;
        let rdr = io::BufReader::new(fh);
        Ok(TdmsFileHandle {
            handle: rdr,
            endianess: Endianess::LittleEndian,
        })
    }

    /// Reads data into the DataType enum based on the value of DataTypeRaw.
    /// The distinction exists because an enum can't have both a defined reprsentation
    /// and a wrapped value
    pub fn read_datatype(&mut self, rawtype: DataTypeRaw) -> Result<DataType, io::Error> {
        let dataout = match rawtype {
            DataTypeRaw::TdmsString => {
                let str_len = match_read_u32(self)?;
                // println!("DBG: Str Len {}", str_len);
                let mut str_raw_buf = vec![0u8; str_len as usize];
                self.handle.read_exact(&mut str_raw_buf)?;
                DataType::TdmsString(
                    String::from_utf8(str_raw_buf).expect("Could not convert from string"),
                )
            }
            DataTypeRaw::U8 => {
                let value = self.handle.read_u8()?;
                DataType::U8(value)
            }
            DataTypeRaw::U16 => {
                let value = match self.endianess {
                    Endianess::BigEndian => self.handle.read_u16::<BigEndian>()?,
                    Endianess::LittleEndian => self.handle.read_u16::<LittleEndian>()?,
                };
                DataType::U16(value)
            }
            DataTypeRaw::U32 => {
                let value = match_read_u32(self)?;
                DataType::U32(value)
            }
            DataTypeRaw::U64 => {
                let value = match_read_u64(self)?;
                DataType::U64(value)
            }
            DataTypeRaw::I8 => {
                let value = self.handle.read_i8()?;
                DataType::I8(value)
            }
            DataTypeRaw::I16 => {
                let value = match self.endianess {
                    Endianess::BigEndian => self.handle.read_i16::<BigEndian>()?,
                    Endianess::LittleEndian => self.handle.read_i16::<LittleEndian>()?,
                };
                DataType::I16(value)
            }
            DataTypeRaw::I32 => {
                let value = match self.endianess {
                    Endianess::BigEndian => self.handle.read_i32::<BigEndian>()?,
                    Endianess::LittleEndian => self.handle.read_i32::<LittleEndian>()?,
                };
                DataType::I32(value)
            }
            DataTypeRaw::I64 => {
                let value = match self.endianess {
                    Endianess::BigEndian => self.handle.read_i64::<BigEndian>()?,
                    Endianess::LittleEndian => self.handle.read_i64::<LittleEndian>()?,
                };
                DataType::I64(value)
            }
            DataTypeRaw::SingleFloat => {
                let value = match self.endianess {
                    Endianess::BigEndian => self.handle.read_f32::<BigEndian>()?,
                    Endianess::LittleEndian => self.handle.read_f32::<LittleEndian>()?,
                };
                DataType::Float(value)
            }
            DataTypeRaw::DoubleFloat => {
                let value = match self.endianess {
                    Endianess::BigEndian => self.handle.read_f64::<BigEndian>()?,
                    Endianess::LittleEndian => self.handle.read_f64::<LittleEndian>()?,
                };
                DataType::Double(value)
            }
            _ => DataType::Void, // TODO this is a dirty placeholder for compilation purposes
        };
        Ok(dataout)
    }

    /// Reads an array of the same type of data into a vector. It's designed to be used
    /// after a complete map of the read operations has been compiled (hence "read_pairs" argument)
    ///
    /// QUESTION: Is there a better way to make a generic read operation than matching on
    /// everything all the time? It feels extremely wasteful.
    pub fn read_data_vector(
        &mut self,
        read_pairs: Vec<ReadPair>,
        rawtype: DataTypeRaw,
    ) -> Result<DataTypeVec, io::Error> {
        // This only works for string initially as I really don't want to type out
        // all that boiler plate but don't know how to make it generic more easily
        let datavec: DataTypeVec = match rawtype {
            DataTypeRaw::TdmsString => {
                let mut datavec: Vec<String> = Vec::new();
                for pair in read_pairs {
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    // This is so convoluted, I already know what the data type is, why do I have
                    // to match it again, don't know how to fix this and keep the simplicity of
                    // read_datatype

                    //NOTE: This also does not actually handle arrays of data i.e. when ReadPair.no_elements > 1
                    // It could be trivially extended with a for loop, but repeated calls to bufrdr might
                    // not be flash. QUESTION: Would it be better to modify read_datatype so it can take
                    // an arguments for the number of reads to perform? In which case might it not become "read_data_vector"?
                    match self.read_datatype(rawtype)? {
                        DataType::TdmsString(string) => datavec.push(string),
                        _ => (),
                    };
                }
                DataTypeVec::TdmsString(datavec)
            }
            _ => DataTypeVec::Void, // Stump implementation until I can get some feedback on generics
        };

        Ok(datavec)
    }
}

/// Represents the contents of a Tdms file
/// Maintains additional meta data about the file extracted from the table of contents (ToC) mask.
#[derive(Debug)]
pub struct TdmsFile {
    handle: TdmsFileHandle,
    segments: Vec<TdmsSegment>,
    interleaved: Interleaved,
    object_paths: BTreeMap<String, u8>, //u8 value is meaningless
}

impl TdmsFile {
    /// Open a Tdms file and initialize a buf rdr to handle access.
    pub fn new_file(path: &path::Path) -> Result<TdmsFile, io::Error> {
        Ok(TdmsFile {
            handle: TdmsFileHandle::open(path)?,
            segments: Vec::new(),
            interleaved: Interleaved::Regular,
            object_paths: BTreeMap::new(),
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
            // Try read in a segment, if an error is returned, intercept it if it's unexpected EoF
            // which indicates there's nothing at the target segment address, or bubble it up
            // if it's a different kind of error.
            let segment = match TdmsSegment::new(self, segment_address) {
                Ok(segment) => segment,
                Err(err) => match err {
                    TdmsError::Io(err) => match err.kind() {
                        ErrorKind::UnexpectedEof => {
                            println!("Completed read");
                            return Ok(self);
                        }
                        _ => return Err(TdmsError::Io(err)), // Any other io error, repackage it and send it on
                    },
                    _ => return Err(err), // Return early on weird custom errors as well
                },
            };

            // TODO I think the early return for malformed segments could happen here?
            // reverse the logical check and return if true (report an error?)
            if segment.next_seg_offset != 0xFFFF_FFFF {
                // note that next segment offset is the total number of bytes in the segment minus the lead in of 28 bytes
                segment_address = segment.next_seg_offset + segment_address + 28;
            }
            self.segments.push(segment);
        }
        Ok(self)
    }

    // Result<Vec<u64>, TdmsError>
    pub fn load_data(&mut self, path: &str) -> Result<DataTypeVec, TdmsError> {
        // I hate having this default
        // TODO if the default above is required, have to guard against it.
        let mut raw_data_type: DataTypeRaw = DataTypeRaw::Void;

        let mut offset: u64 = 0;
        let mut chunk_size: u64 = 0;
        let mut size: u64 = 0;
        let mut no_vals: u64 = 0;

        let mut data: DataTypeVec = DataTypeVec::Void;

        // Dive into the segments and then into the meta data
        // Attempt to index out the requested object and gather
        // information required to read its raw data.
        for segment in &self.segments {
            segment.meta_data.as_ref().map(|meta_data| {
                meta_data
                    .objects
                    .get(path) // returns option containing reference
                    .map(|object| {
                        match object.raw_data_type {
                            Some(data_type) => raw_data_type = data_type,
                            None => (),
                        };
                        match object.total_size {
                            Some(total_size) => size = total_size,
                            None => (),
                        };
                        match object.no_raw_vals {
                            Some(no_raw_vals) => no_vals = no_raw_vals,
                            None => (),
                        };
                    });
                offset = meta_data.prev_obj_sizes.get(path).unwrap().clone();
                chunk_size = meta_data.chunk_size;
            });

            // println!("DBG start_index: {}", segment.start_index);
            // println!("DBG raw_data: {}", segment.raw_data_offset);
            // println!("DBG chunk_size: {}", chunk_size);
            // println!("DBG chan offset: {}", offset);

            let mut read_pairs: Vec<ReadPair> = Vec::new();

            for i in 0..segment.no_chunks {
                let pair = ReadPair {
                    start_index: segment.start_index
                        + 28
                        + segment.raw_data_offset
                        + i * chunk_size
                        + offset,
                    no_elements: no_vals,
                };
                read_pairs.push(pair);
            }
            data = self.handle.read_data_vector(read_pairs, raw_data_type)?;
        }

        Ok(data)
    }

    /// Return a vector of channel paths
    pub fn objects(&self) -> Vec<&String> {
        let mut objects: Vec<&String> = Vec::new();

        for key in self.object_paths.keys() {
            objects.push(key)
        }
        objects
    }
}

/// A TdmsSegment consists of a 28 byte lead in followed by a series of optional MetaData properties
/// This is followed in turn by raw data if it exists.
#[derive(Debug)]
pub struct TdmsSegment {
    // Segment lead in data is 28 bytes long
    file_tag: u32,
    toc_mask: u32,
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
    /// Load in a segment and parse all objects and properties, does not load raw data. This allows lazy loading to handle large files.
    fn new(file: &mut TdmsFile, index: u64) -> Result<TdmsSegment, TdmsError> {
        // Seek to the "absolute index" (relative to start) This index has to be built up for each segment as we go. This is handled in the
        // map_segments function
        let target_loc = file.handle.handle.seek(SeekFrom::Start(index))?;
        println!("Target Loc: {:x}", target_loc);

        // Convert the critical lead in information to appropriate representation
        let file_tag: u32 = file.handle.handle.read_u32::<BigEndian>()?;
        let toc_mask: u32 = file.handle.handle.read_u32::<LittleEndian>()?;

        if toc_mask & TocProperties::KTocBigEndian as u32 == TocProperties::KTocBigEndian as u32 {
            file.handle.endianess = Endianess::BigEndian;
        }

        // Finish out the lead in based on whether the data is little endian
        let version_no = match_read_u32(&mut file.handle)?;
        let next_seg_offset = match_read_u64(&mut file.handle)?;
        let raw_data_offset = match_read_u64(&mut file.handle)?;

        let current_loc = file.handle.handle.seek(SeekFrom::Current(0))?; // position at end of lead in read

        // Load the meta_data for this segment TODO 2) does there need to be a check of kToCNewContents?
        let meta_data = TdmsMetaData::new(file)?;
        let no_chunks = (next_seg_offset - raw_data_offset) / meta_data.chunk_size;
        let meta_data = Some(meta_data);

        // Load the raw data, this part should move out to a different function
        // let mut raw_data = vec![0u8; (next_seg_offset - raw_data_offset) as usize];
        // file.handle.read_exact(&mut raw_data)?;
        let raw_data = None;

        // Initialise the Segment
        let segment = TdmsSegment {
            start_index: index,
            file_tag,
            toc_mask,
            version_no,
            next_seg_offset,
            raw_data_offset,
            meta_data,
            raw_data,
            no_chunks,
        };

        println!("__SEGMENT__");
        println!("{}", segment);
        println!("Current Loc: {:x}", current_loc);
        match &segment.meta_data {
            Some(meta_data) => {
                println!("__METADATA__");
                println!("{}", meta_data);
            }
            None => (),
        }

        Ok(segment)
    }
}

#[derive(Debug)]
pub struct TdmsMetaData {
    no_objects: u32,
    objects: BTreeMap<String, TdmsObject>,
    chunk_size: u64,
    // This is a helper map to figure out how deep into any given raw data chunk to start reading
    // the values for the object of interest
    prev_obj_sizes: BTreeMap<String, u64>,
}

impl fmt::Display for TdmsMetaData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "No. objects:\t{}", self.no_objects)?;
        writeln!(f, "Chunk Size:\t{}", self.chunk_size)?;
        for (k, v) in &self.prev_obj_sizes {
            writeln!(f, "Obj: {:?}\t\tPrev Size: {}", k, v)?;
        }

        for (_, obj) in &self.objects {
            writeln!(f, "__Object__")?;
            write!(f, "{}", obj)?;
        }

        Ok(())
    }
}

impl TdmsMetaData {
    /// Creates a new meta data struct and reads objects into it.
    pub fn new(file: &mut TdmsFile) -> Result<TdmsMetaData, TdmsError> {
        Ok(TdmsMetaData::_new(&mut file.handle)?._read_meta_data(file)?)
    }

    fn _new(file: &mut TdmsFileHandle) -> Result<TdmsMetaData, TdmsError> {
        let no_objects = match_read_u32(file)?;
        Ok(TdmsMetaData {
            no_objects,
            objects: BTreeMap::new(),
            chunk_size: 0,
            prev_obj_sizes: BTreeMap::new(),
        })
    }

    /// Read in objects, keep track of chunk size so objects can be loaded later by
    /// directly addressing their constituents
    fn _read_meta_data(mut self, file: &mut TdmsFile) -> Result<TdmsMetaData, TdmsError> {
        let mut chunk_size: u64 = 0;
        for _i in 0..self.no_objects {
            let obj = TdmsObject::read_object(file)?;
            self.prev_obj_sizes
                .insert(obj.object_path.clone(), chunk_size);
            match obj.total_size {
                Some(size) => chunk_size = chunk_size + size,
                None => chunk_size = chunk_size,
            }

            self.objects.insert(obj.object_path.clone(), obj);
        }
        self.chunk_size = chunk_size;
        Ok(self)
    }
}

#[derive(Debug)]
pub struct TdmsObject {
    object_path_len: u32,
    object_path: String,
    raw_data_index: u32,
    raw_data_type: Option<DataTypeRaw>, // present depending on raw_data_index val
    raw_data_dim: Option<u32>,
    no_raw_vals: Option<u64>,
    total_size: Option<u64>, // in bytes, variable length datatypes only e.g. string
    no_properties: u32,
    properties: Option<Vec<ObjectProperty>>,
}

impl fmt::Display for TdmsObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // writeln!(f, "Obj path len:\t{}", self.object_path_len)?;
        writeln!(f, "Obj path:\t{}", self.object_path)?;
        writeln!(f, "Raw data index:\t{:x}", self.raw_data_index)?;
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

#[derive(Debug)]
pub struct ObjectProperty {
    prop_name_len: u32,
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
    /// Currently a bit twisted as it requires the full file structure to look back at
    /// previous information. QUESTION: Is there a better division of responsibility which
    /// avoids this problem
    pub fn read_object(file: &mut TdmsFile) -> Result<TdmsObject, TdmsError> {
        let path_len = match_read_u32(&mut file.handle)?;
        // println!("DBG: Path len:  {}", path_len);

        let mut path = vec![0u8; path_len as usize];
        // let current_loc = file.handle.seek(SeekFrom::Current(0))?;
        // println!("Current Loc: {:x}", current_loc);

        file.handle.handle.read_exact(&mut path)?;
        // TODO: The below error handling sucks, should convert TdmsError so it wraps string parse errors as well
        let path =
            String::from_utf8(path).or(Err("Unable to convert buffer to string".to_string()))?;
        // println!("DBG: Path:  {:?}", path);

        file.object_paths.insert(path.clone(), 0);

        let mut raw_data_index = match_read_u32(&mut file.handle)?;
        // println!("DBG: data_index:  {:?}", raw_data_index);
        let raw_data_type;
        let raw_data_dim;
        let no_raw_vals;
        let total_size;

        if raw_data_index == 0xFFFF_FFFF {
            // No raw data in this object
            raw_data_type = None;
            raw_data_dim = None;
            no_raw_vals = None;
            total_size = None;
        } else if raw_data_index == 0 {
            // raw data for this object is identical to previous segments, copy the raw data across
            // I'm using map_or here to perform a kind of unwrap with fail back, the None case should never
            // be triggered. QUESTION: possibly make it an explicit failure, not function to use map_or_err?
            let previous_object = file
                .segments
                .last()
                .map_or(None, |segment| {
                    segment
                        .meta_data
                        .as_ref()
                        .map_or(None, |metadata| metadata.objects.get(&path))
                })
                .unwrap(); // I'm not sure of a more graceful way of avoiding this i.e. what errors need to be considered here.
            raw_data_type = previous_object.raw_data_type;
            raw_data_dim = previous_object.raw_data_dim;
            no_raw_vals = previous_object.no_raw_vals;
            total_size = previous_object.total_size;
            raw_data_index = previous_object.raw_data_index;
        } else {
            raw_data_type = num::FromPrimitive::from_u32(match_read_u32(&mut file.handle)?);
            raw_data_dim = Some(match_read_u32(&mut file.handle)?);
            no_raw_vals = Some(match_read_u64(&mut file.handle)?);
            total_size = Some(match_read_u64(&mut file.handle)?);
        };
        // println!("DBG: data_type:  {:?}", raw_data_type);
        // println!("DBG: data_dim:  {:?}", raw_data_dim);
        // println!("DBG: no_vals:  {:?}", no_raw_vals);
        // println!("DBG: total_size:  {:?}", total_size);

        // Read the object properties
        let no_properties = match_read_u32(&mut file.handle)?;
        let properties: Option<Vec<ObjectProperty>>;
        if no_properties > 0 {
            let mut temp_vec = Vec::new();
            for _i in 0..no_properties {
                temp_vec.push(ObjectProperty::read_property(&mut file.handle)?);
            }
            properties = Some(temp_vec);
        } else {
            properties = None;
        }

        Ok(TdmsObject {
            object_path_len: path_len,
            object_path: path,
            raw_data_index,
            raw_data_type,
            raw_data_dim,
            no_raw_vals,
            total_size,
            no_properties,
            properties,
        })
    }
}

impl ObjectProperty {
    /// Read properties associated with an object
    pub fn read_property(file: &mut TdmsFileHandle) -> Result<ObjectProperty, io::Error> {
        let prop_name_len = match_read_u32(file)?;

        let mut prop_name = vec![0u8; prop_name_len as usize];
        file.handle.read_exact(&mut prop_name)?;
        // Again, should convert TdmsError to wrap string parse errors
        let prop_name = String::from_utf8(prop_name).expect("Unable to convert buffer to string");
        // QUESTION: I struggled to make this a one liner, something in the background kept
        // wrapping Option around the result, regardless of whehter I called unwrap
        // QUESTION: Is there a better way to map raw values to enum than the approach I have taken?
        let prop_datatype = num::FromPrimitive::from_u32(match_read_u32(file)?);
        let prop_datatype = prop_datatype.unwrap();

        let property = file.read_datatype(prop_datatype)?;

        Ok(ObjectProperty {
            prop_name_len,
            prop_name,
            data_type: prop_datatype,
            property,
        })
    }
}

fn main() -> Result<(), TdmsError> {
    // call with cargo run Example.tdms to run the example
    let args: Vec<String> = env::args().collect();

    println!("{:?}", args);

    let fname = &args[1];

    // Convert string to path and load file handle into struct
    let pathbuf = path::PathBuf::from(fname);

    println!();
    println!("-----------------------------------------");
    println!("Loading TDMS File {}", fname);
    println!("-----------------------------------------");
    println!();
    let mut tdms_file = TdmsFile::new_file(&pathbuf)?;

    tdms_file.map_segments()?;

    let channels = tdms_file.objects();
    for channel in channels {
        println!("{:?}", channel);
    }

    let data = tdms_file.load_data("/'Untitled'/'Time Stamp'")?;
    println!("{}", data);

    Ok(())
}
