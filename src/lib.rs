use indexmap::{IndexMap};
use std::fmt;
use std::fs;
use std::io;
use std::io::BufReader;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path;

use byteorder::{BE, LE, *};
use log::debug;
pub mod tdms_datatypes;
pub use tdms_datatypes::{DataType, DataTypeRaw, DataTypeVec, TocProperties};
pub mod tdms_error;
pub use tdms_error::{TdmsError, TdmsErrorKind};

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
/// ReadPairs give the absolute file index, and the #no of bytes to read at that index, a channel is accessed by a vector of ReadPairs
pub struct ReadPair {
    start_index: u64,
    no_bytes: u64,
}

impl fmt::Display for ReadPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "start: {}\t no_bytes: {}", self.start_index, self.no_bytes)?;        
        Ok(())
    }
}

/// A struct to maintain the vector of read pairs associated with a particular object (channel), as well as keep track of the object and any properties it accrues throughout the reading process. The set of maps for each object are maintained within the main "TdmsFile" struct via a hash map.
#[derive(Debug, Clone)]
pub struct ObjectMap {
    last_object: TdmsObject, // the most up to date version of the object, properties and indexing information are copied to this.
    read_map: Vec<ReadPair>, // for each segment in the file a vector of read pairs exist.
    total_bytes: u64, // The total byte count of raw data associated with the object, for allocating vectors to dump the data into.
    bigendian: bool, // whether the object associated with this map has been logged as bigendian 
}

impl fmt::Display for ObjectMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Object:\t{}", self.last_object)?;
        
        Ok(())
    }
}
/// Represents the contents of a Tdms file which consists of a series  of segments + ancillary data which is created to index those segments.
#[derive(Debug)]
pub struct TdmsFile {
    handle: io::BufReader<std::fs::File>,
    segments: Vec<TdmsSegment>,    
    all_objects: IndexMap<String, ObjectMap>, // Keeps track of all objects in file and their read maps, order not important for this one, using indexmap to avoid running multiple hashmap types.
    live_objects: IndexMap<String, ObjectMap>, // Keeps track of order and data size of objects accumulated over segments, is reset when kToCNewObjectList flag is detected
}

/// Helper function for reading string.
pub fn read_string<O: ByteOrder>(reader: &mut BufReader<fs::File>) -> Result<String, TdmsError> {
    let str_len = reader.read_u32::<O>()?;

    let mut str_raw_buf = vec![0u8; str_len as usize];
    reader.read_exact(&mut str_raw_buf)?;
    Ok(String::from_utf8(str_raw_buf)?)
}  

impl TdmsFile {
    /// Open a Tdms file and initialize a buf rdr to handle access.
    pub fn open(path: &path::Path) -> Result<TdmsFile, TdmsError> {
        let mut file = TdmsFile::new(path)?;
        file.map_segments()?;
        Ok(file)

    }

    fn new(path: &path::Path) -> Result<TdmsFile, io::Error> {
        let fh = fs::File::open(path)?;
        let rdr = io::BufReader::new(fh);
        Ok(TdmsFile {
            handle: rdr,
            segments: Vec::new(),            
            all_objects: IndexMap::new(),
            live_objects: IndexMap::new(),
        })
    }

    
      
    
        /// Reads data into the DataType enum based on the value of DataTypeRaw.
        /// The distinction exists because an enum can't have both a defined representation
        /// i.e. an integer value indicating which enum value it is, and a wrapped value
    pub fn read_datatype<O: ByteOrder>(&mut self, rawtype: DataTypeRaw) -> Result<DataType, TdmsError> {
        let dataout = match rawtype {
                DataTypeRaw::TdmsString => DataType::TdmsString(read_string::<O>(&mut self.handle)?),
                DataTypeRaw::U8 => DataType::U8(self.handle.read_u8()?),
                DataTypeRaw::U16 => DataType::U16(self.handle.read_u16::<O>()?),
                DataTypeRaw::U32 => DataType::U32(self.handle.read_u32::<O>()?),
                DataTypeRaw::U64 => DataType::U64(self.handle.read_u64::<O>()?),
                DataTypeRaw::I8 => DataType::I8(self.handle.read_i8()?),
                DataTypeRaw::I16 => DataType::I16(self.handle.read_i16::<O>()?),
                DataTypeRaw::I32 => DataType::I32(self.handle.read_i32::<O>()?),
                DataTypeRaw::I64 => DataType::I64(self.handle.read_i64::<O>()?),
                DataTypeRaw::SingleFloat => DataType::Float(self.handle.read_f32::<O>()?),
                DataTypeRaw::DoubleFloat => DataType::Double(self.handle.read_f64::<O>()?),
                DataTypeRaw::Boolean => DataType::Boolean(match self.handle.read_u8()? {
                                                0 => false,
                                                _ => true,
                                            }),                     
                DataTypeRaw::TimeStamp => {
                    let epoch = self.handle.read_i64::<O>()?;
                    let radix = self.handle.read_u64::<O>()?;
                    DataType::TimeStamp(tdms_datatypes::TdmsTimeStamp { epoch, radix })
                }
                _ => DataType::Void(()), // TODO this is a dirty placeholder
                };          
        
        Ok(dataout)
    }

    /// Reads an array of the same type of data into a vector. It's designed to be used
    /// after a complete map of the read operations has been compiled via the map_segments function
    /// 
    /// IMPORTANT NOTE: Due to the default buffer size of BufRdr (8kb) it might not be more
    /// efficient to try and lazy load channels in the long run, as repeated seek operations at 
    /// the file system level must be performed if data is spaced more than 8kb's apart.
    ///
    /// QUESTION: Is there a better way to make a generic read operation than matching on
    /// everything all the time? It feels extremely wasteful.

    // TODO: Needs MAJOR work
    #[rustfmt::skip]
    pub fn read_data_vector<O: ByteOrder>(&mut self, path: &str) -> Result<DataTypeVec, TdmsError> {
        let object_map = self.all_objects.get(path).ok_or(TdmsError { kind: TdmsErrorKind::ChannelNotFound})?;
        let read_pairs = &object_map.read_map;
        let rawtype = &object_map.last_object.raw_data_type.ok_or(TdmsError {kind: TdmsErrorKind::ObjectHasNoRawData})?;
        let total_bytes = &object_map.total_bytes;       
        

        let datavec: DataTypeVec = match rawtype {
            DataTypeRaw::Boolean => {
                let mut datavec: Vec<bool> = Vec::new();
                for pair in read_pairs {
                    self.handle.seek(SeekFrom::Start(pair.start_index))?; 
                    datavec.push(match self.handle.read_u8()? {
                        0 => false,
                        _ => true,
                    })
                }
                DataTypeVec::Boolean(datavec)
            }
            DataTypeRaw::I8 => {
                let mut datavec: Vec<i8> = Vec::new();
                for pair in read_pairs {
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;                    
                    datavec.push(self.handle.read_i8()?);
                }
                DataTypeVec::I8(datavec)
            }
            DataTypeRaw::I16 => {  
                let mut datavec: Vec<i16> = vec![0; (total_bytes/2) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 2 ;
                    self.handle.read_i16_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::I16(datavec)
            }
            DataTypeRaw::I32 => {  
                let mut datavec: Vec<i32> = vec![0; (total_bytes/4) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 4 ;
                    self.handle.read_i32_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::I32(datavec)
            }
            DataTypeRaw::I64 => {  
                let mut datavec: Vec<i64> = vec![0; (total_bytes/8) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 8 ;
                    self.handle.read_i64_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::I64(datavec)
            }
            DataTypeRaw::U8 => {
                let mut datavec: Vec<u8> = Vec::new();
                for pair in read_pairs {
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;                    
                    datavec.push(self.handle.read_u8()?);
                }
                DataTypeVec::U8(datavec)
            }
            DataTypeRaw::U16 => {  
                let mut datavec: Vec<u16> = vec![0; (total_bytes/2) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 2 ;
                    self.handle.read_u16_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::U16(datavec)
            }
            DataTypeRaw::U32 => {  
                let mut datavec: Vec<u32> = vec![0; (total_bytes/4) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 4 ;
                    self.handle.read_u32_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::U32(datavec)
            }
            DataTypeRaw::U64 => {  
                let mut datavec: Vec<u64> = vec![0; (total_bytes/8) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 8 ;
                    self.handle.read_u64_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::U64(datavec)
            }
            DataTypeRaw::TdmsString => {
                let mut datavec: Vec<String> = Vec::new();
                for pair in read_pairs {
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;                    
                    datavec.push(read_string::<O>(&mut self.handle)?);
                }
                DataTypeVec::TdmsString(datavec)
            }
            DataTypeRaw::SingleFloat => {  
                let mut datavec: Vec<f32> = vec![0.0; (total_bytes/4) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 4 ;
                    self.handle.read_f32_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::Float(datavec)
            }
            DataTypeRaw::DoubleFloat => {  
                let mut datavec: Vec<f64> = vec![0.0; (total_bytes/8) as usize];
                let mut i: usize = 0; // dummy variable to track bytes for indexing               
                for pair in read_pairs {                     
                    self.handle.seek(SeekFrom::Start(pair.start_index))?;
                    let no_values = pair.no_bytes as usize / 8 ;
                    self.handle.read_f64_into::<O>(&mut datavec[i..i+no_values])?;
                    i += no_values;                                      
                }
                DataTypeVec::Double(datavec)
            }
            _ => DataTypeVec::Void(Vec::new()), // Stump implementation until I can get some feedback on generics
        };        
        Ok(datavec)
    }

    /// Walk the file attempting to load the segment meta data and objects.
    /// Raw data is not loaded during these reads in the interest of Lazy Loading
    /// i.e. graceful handling of very large files.
    fn map_segments(&mut self) -> Result<&mut Self, TdmsError> {
        // TODO: The construction of this function isn't right, if segment address ever is
        // 0xFFFF_FFFF then the file is malformed and this should probably be some kind of error.
        let mut segment_address = 0;
        while segment_address != 0xFFFF_FFFF {
            // Try read in a segment, if an error is returned, intercept it if it's
            // unexpected EoF which indicates there's nothing at the target segment
            // address, or bubble it up if it's a different kind of error.
            debug!("=============NEW SEGMENT==============");
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
    pub fn load_data(&mut self, path: &str) -> Result<DataTypeVec, TdmsError> {        
        // check if object exists in map
        if self.all_objects.get(path).ok_or(TdmsError { kind: TdmsErrorKind::ChannelNotFound})?.bigendian {
            Ok(self.read_data_vector::<BE>(path)?)
        } else {
            Ok(self.read_data_vector::<LE>(path)?)
        }

    }

    /// Return a vector of channel paths
    pub fn objects(&self) -> Vec<&str> {
        let mut objects: Vec<&str> = Vec::new();

        for key in self.all_objects.keys() {
            objects.push(key)
        }
        objects
    }

    /// Display an objects properties
    pub fn object_properties(&self, path: &str) -> Result<(), TdmsError> {
        let object = self.all_objects.get(path).ok_or(TdmsError { kind: TdmsErrorKind::ChannelNotFound})?;

        print!("{}", object.last_object);

        Ok(())
    }

    /// Print an object's read pairs
    pub fn object_with_read_pairs(&self, path: &str) -> Result<(), TdmsError> {
        let object = self.all_objects.get(path).ok_or(TdmsError { kind: TdmsErrorKind::ChannelNotFound})?;
        
        print!("{}", object);        
        Ok(())
    }

    /// Diagnostic function to print current location for debugging purposes
    pub fn current_loc(&mut self) {
        println!("{:?}", self.handle.seek(SeekFrom::Current(0)));
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
    bigendian: bool,
    new_obj_list: bool,
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
    /// construct a TDMS segment
    fn new(file: &mut TdmsFile, index: u64) -> Result<TdmsSegment, TdmsError> {
        TdmsSegment::_new(index)._read(file)
    }

    fn _new(index: u64) -> TdmsSegment {
        TdmsSegment {
            start_index: index,
            file_tag: 0,
            toc_mask: 0,
            contains_metadata: false,
            contains_rawdata: false, 
            contains_daqmx: false,
            interleaved: false,
            bigendian: false,
            new_obj_list: false,
            version_no: 0,        
            next_seg_offset: 0,
            raw_data_offset: 0,
            meta_data: None,
            raw_data: None,
            no_chunks: 0,
        }
    }

    /// Load in a segment and parse all objects and properties, does not load raw data. This allows lazy loading to handle very large files.
    fn _read(mut self, file: &mut TdmsFile) -> Result<TdmsSegment, TdmsError> {
        // Seek to the "absolute index" (relative to start) This index has to be built up for each segment as we go. This is handled in the map_segments function
        let target_loc = file.handle.seek(SeekFrom::Start(self.start_index))?;
        debug!("Target Loc: {}", target_loc);
        
        self.read_lead_in(file)?;

        let current_loc = file.handle.seek(SeekFrom::Current(0))?; // position at end of lead in read
        debug!("current_loc: {}", current_loc);

        if !self.contains_metadata {
           // no new meta data, copy previous meta-data, I don't think this possibility can actually eventuate if theres no metadata it's because all objects were configured the same and data has been appended to the prior segment as a new chunk
        }

        // Return the initialised Segment with lead in and metadata
        Ok(self)
    }

    fn read_lead_in(&mut self, file: &mut TdmsFile) -> Result<&mut TdmsSegment, TdmsError> {
        // Convert the critical lead in information to appropriate representation, we know the 
        // first part of the lead in is little endian so we save a check here.
        let file_tag: u32 = file.handle.read_u32::<LE>()?;
        let toc_mask: u32 = file.handle.read_u32::<LE>()?;

        debug!("File tag: {}", file_tag);
        debug!("toc_mask: {:b}", toc_mask);

        if (toc_mask & TocProperties::KTocMetaData as u32) != 0 {
            self.contains_metadata = true            
        }
        debug!("Contains metadata?: {}", self.contains_metadata);

        if (toc_mask & TocProperties::KTocRawData as u32) != 0 {
            self.contains_rawdata = true        
        }
        debug!("Contains rawdata?: {}", self.contains_rawdata);

        if (toc_mask & TocProperties::KTocDAQmxRawData as u32) != 0 {
            self.contains_daqmx = true
        }
        debug!("Contains daqmxdata?: {}", self.contains_daqmx);

        if (toc_mask & TocProperties::KTocInterleavedData as u32) != 0 {
            self.interleaved = true
        }
        debug!("Interleaved?: {}", self.interleaved);

        if (toc_mask & TocProperties::KTocBigEndian as u32) != 0 {
            self.bigendian = true
        }
        debug!("Endianess: {:?}", self.bigendian);

        if (toc_mask & TocProperties::KTocNewObjList as u32) != 0 {
            self.new_obj_list = true
        }
        debug!("Contains new objects?: {}", self.new_obj_list);

        if self.bigendian {
            self.read_metadata::<BE>(file)?;
        } else {
            self.read_metadata::<LE>(file)?;
        }        

        Ok(self)
    }

    fn read_metadata<O: ByteOrder>(&mut self, file: &mut TdmsFile) -> Result<&mut TdmsSegment, TdmsError> {
        // Finish out the lead in based on whether the data is little endian
        self.version_no = file.handle.read_u32::<O>()?;
        self.next_seg_offset = file.handle.read_u64::<O>()?;
        self.raw_data_offset = file.handle.read_u64::<O>()?;
        debug!("version_no: {}", self.version_no);
        debug!("next_seg_offset: {}", self.next_seg_offset);
        debug!("raw_data_offset: {}", self.raw_data_offset);

        // Load the meta_data for this segment 
        let mut meta_data = TdmsMetaData::read_metadata::<O>(file)?;

        // Update the object maps
        // TODO: This still does not handle interleaved data at all
        if self.new_obj_list {
            // if new_obj list has been set, then the chunk size as reported by new metadata is everything and we could have a totally new ordering of data for this segment. This will reset the live_objects map
            debug!("chunk size: {}", meta_data.chunk_size);            
            let no_chunks: u64 = if meta_data.chunk_size > 0 {
                (self.next_seg_offset - self.raw_data_offset) / meta_data.chunk_size
            } else {
                0
            };        
            debug!("no_chunks: {}", no_chunks);
            self.no_chunks = no_chunks;

            // create new map of objects
            let mut new_map: IndexMap<String, ObjectMap> = IndexMap::new();
            
                 
            let mut relative_position: u64 = 0; // Used in computing read pairs as we go
            for object in meta_data.objects.iter() {
                //compute read pairs as we go to save double iteration over the objects map, only compute if size here is > 0
                let mut new_read_map: Vec<ReadPair> = Vec::new();
                if object.total_size > 0 {                    
                    for i in 0..no_chunks {
                        let pair = ReadPair {
                            start_index: self.start_index + 28 + self.raw_data_offset + i * meta_data.chunk_size + relative_position,
                            no_bytes: object.total_size,
                        };
                        new_read_map.push(pair);
                    }
                }

                // pull objects by key using new object list and update then insert updated or new objects into new objectmap
                if let Some((key, mut object_map)) = file.all_objects.remove_entry(&object.object_path) {
                    // if the object has a previous entry update it and push to new map, we remove here so that we don't miss any objects that aren't in the new list. At the end we'll append those on.
                    object_map.last_object = object.clone();
                    object_map.read_map.append(&mut new_read_map);
                    object_map.total_bytes += object.total_size;
                    new_map.insert(key, object_map);
                } else {
                    // push the new object
                    new_map.insert(object.object_path.clone(), 
                    ObjectMap { 
                        last_object: object.clone(), 
                        read_map: new_read_map, 
                        total_bytes: object.total_size,
                        bigendian: self.bigendian});                    
                }
                relative_position += object.total_size;
            }           
            
            // At this point all objects are into a map in their correct order, update the live_objects map for future use.
            file.live_objects = new_map.clone();
            
            // at this point we've re-ordered objects that are recorded in the new list, but we want to keep around old objects which weren't in this segment as well, so we join what's left in all_objects and update the old map
            // new_map.extend(file.all_objects.drain(..));
            // file.all_objects = new_map;
            file.all_objects.extend(new_map);

        } else {
            // Need to iterate over new list of objects, check if it's in live objects and update, otherwise append it to live objects under a new key.
            for object in meta_data.objects.iter() {                
                // Check if it's in the live_objects map and update otherwise insert (presumably at end)
                let mut existing_object = file.live_objects.entry(object.object_path.clone()).or_insert(ObjectMap { 
                    last_object: object.clone(), 
                    read_map: Vec::new(), 
                    total_bytes: object.total_size,
                    bigendian: self.bigendian});
                
                // Update the entry with the current instance of the object, along with the new total size for this object, leave the readmap as we'll update it later
                existing_object.last_object = object.clone();
                existing_object.total_bytes += object.total_size;
                
            }
            
            // Iterate over the up to date live_objects list and compute new read maps
            let mut new_chunk_size = 0;
            

            // First we have to establish the correct chunk_size computation accounting for all live_objects
            for (_key, object_map) in file.live_objects.iter_mut() {
                new_chunk_size += object_map.last_object.total_size; 
            }

            // Because of the way it was computed, meta_data chunk size was incorrectly calculated prior to this point (only accounted for new objects), update with the comprehensive calc
            meta_data.chunk_size += new_chunk_size;

            let no_chunks: u64 = if meta_data.chunk_size > 0 {
                (self.next_seg_offset - self.raw_data_offset) / meta_data.chunk_size
            } else {
                0
            };        
            debug!("no_chunks: {}", no_chunks);
            self.no_chunks = no_chunks;

            // Now we can go over it again and calculate the new read_map points for the segment
            // TODO (Distant future): There must be a way to stop going over and over the list of objects.
            let mut relative_position: u64 = 0; // Used in computing read pairs as we go
            for (key, object_map) in file.live_objects.iter_mut() {
                 // add the new read_pair, only compute if size here is > 0
                if object_map.last_object.total_size > 0 {                    
                    for i in 0..no_chunks {
                        let pair = ReadPair {
                            start_index: self.start_index + 28 + self.raw_data_offset + i * meta_data.chunk_size + relative_position,
                            no_bytes: object_map.last_object.total_size,
                        };
                        object_map.read_map.push(pair);
                    }
                }
                relative_position += object_map.last_object.total_size;

                // Update all_objects as well to prevent them diverging
                file.all_objects.insert(key.clone(), object_map.clone());
            }


        }

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
    /// Read in objects, keep track of accumulating channel size so objects can be loaded
    /// later by directly addressing their constituent addresses
    #[rustfmt::skip]
    pub fn read_metadata<O: ByteOrder>(file: &mut TdmsFile) -> Result<TdmsMetaData, TdmsError> {
        let no_objects = file.handle.read_u32::<O>()?;
        debug!("no_objects: {}", no_objects);

        let mut chunk_size: u64 = 0;
        let mut objects: Vec<TdmsObject> = Vec::new();

        for i in 0..no_objects {
            debug!("-----------------------------------");
            debug!("object #: {}", i);

            // Read in an object including properties
            let obj = TdmsObject::read_object::<O>(file)?;
            
            // Keep track of the accumulating raw data size for objects
            chunk_size += obj.total_size;            
            
            objects.push(obj);
        }
        
        Ok(TdmsMetaData {
            no_objects,
            objects,
            chunk_size,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TdmsObject {
    object_path: String,
    index_info_len: u32, // The length in bytes of the indexing info for raw data, including the length of this field. Should always be 20 (defined length) or 28 (variable length)
    raw_data_type: Option<DataTypeRaw>, // appears in file as u32.
    raw_data_dim: Option<u32>,
    no_raw_vals: Option<u64>,
    total_size: u64, // of raw data in bytes, appears in file for variable length types (String) only. comptued otherwise
    no_properties: u32,
    properties: IndexMap<String, ObjectProperty>,
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
        writeln!(f, "Actual property count:\t{:?}", self.properties.len())?;
        for (_key, property) in self.properties.iter() {
            writeln!(f, "__Property__")?;
            write!(f, "{}", property)?;
        }         

        Ok(())
    }
}

impl TdmsObject {
    /// Read an object from file including its properties    
    pub fn read_object<O: ByteOrder>(file: &mut TdmsFile) -> Result<TdmsObject, TdmsError> {
        let path = read_string::<O>(&mut file.handle)?;
        debug!("obj_path: {}", path);

        // Try to clone the last recorded version of the object from the list of object_map to use as a base, otherwise instantiate a new object
        let mut new_object = match file.all_objects.get(&path) {
            Some(object_map) => object_map.last_object.clone(),
            None => {
                TdmsObject {
                    object_path: path,
                    index_info_len: 0,
                    raw_data_type: None,
                    raw_data_dim: None,
                    no_raw_vals: None,
                    total_size: 0,
                    no_properties: 0,
                    properties: IndexMap::new(),
                }
            },
        };       

        new_object.index_info_len = file.handle.read_u32::<O>()?;
        debug!("- data_index_len:  {:?}", new_object.index_info_len);

        // TODO: Need to handle DAQmx data types here.
        
        if new_object.index_info_len == 0xFFFF_FFFF {
            // No raw data in this object

        } else if new_object.index_info_len == 0 {
            // raw data index for this object is identical to previous segments, this implies that we can use the old object with indexing info as a baseline

        } else if new_object.index_info_len == 0x6912_0000  {
            // DAQmx with format changing scaler.

        } else if new_object.index_info_len == 0x6913_0000 {
            // DAQmx with digital line scaler

        } else  {
            // This is a fresh, non DAQmx object, or new raw data for the object which contains data
            // read a u32 and attempt to convert it to a DataTypeRaw enum value. Propagate an error if this fails.
            let raw_data_type =
                num::FromPrimitive::from_u32(file.handle.read_u32::<O>()?).ok_or(TdmsError {
                    kind: TdmsErrorKind::RawDataTypeNotFound,
                })?;
            // stage the read so we can use the information to compute total size
            let dim = file.handle.read_u32::<O>()?;
            let no_vals = file.handle.read_u64::<O>()?;

            // total_size (bytes) is either recorded in the file if data is TdmsString or else
            // must be computed. Size() will return an error if called on DataTypeRaw::TdmsString which is why there is a guard clause here.
            let total_size = match raw_data_type {
                DataTypeRaw::TdmsString => file.handle.read_u64::<O>()?,
                other => other.size()? * no_vals * dim as u64,
            };
            new_object.raw_data_type = Some(raw_data_type);
            new_object.raw_data_dim = Some(dim);
            new_object.no_raw_vals = Some(no_vals);
            new_object.total_size = total_size;
        };
        debug!("- data_type:  {:?}", new_object.raw_data_type);
        debug!("- data_dim:  {:?}", new_object.raw_data_dim);
        debug!("- no_vals:  {:?}", new_object.no_raw_vals);
        debug!("- total_size:  {:?}", new_object.total_size);
        

        // check what properties exist or have changed 
        new_object.update_properties::<O>(file)?;
        
        Ok(new_object)
    }

    fn update_properties<O: ByteOrder>(&mut self, file: &mut TdmsFile) -> Result<&mut Self, TdmsError> {
        // Read the object properties, update if that property already exists for that object
        self.no_properties = file.handle.read_u32::<O>()?;
        if self.no_properties > 0 {            
            for _i in 0..self.no_properties {
                let property = ObjectProperty::read_property::<O>(file)?;
                debug!("prop_name: {}", property.prop_name);
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
    data_type: DataTypeRaw, // The marker for what type of data this property contains
    property: DataType, // The actual property within a wrapper
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

impl ObjectProperty {    
    /// Instantiate a property and read into it.
    pub fn read_property<O: ByteOrder>(file: &mut TdmsFile) -> Result<ObjectProperty, TdmsError> {       
        let prop_name = read_string::<O>(&mut file.handle)?;
        debug!("prop_name: {}", prop_name);    
        // Read in a u32 and attempt to convert to a variant of DataTypeRaw. Raise an error if this fails.
        let data_type =
            num::FromPrimitive::from_u32(file.handle.read_u32::<O>()?).ok_or(TdmsError {
                kind: TdmsErrorKind::RawDataTypeNotFound,
            })?;
        // debug!("prop_datatype {:?}", prop_datatype);
        let property = file.read_datatype::<O>(data_type)?;
        debug!("property: {:?}", property);
        
        Ok(ObjectProperty {prop_name, data_type, property})
    }
}
