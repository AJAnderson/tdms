use std::io::{Read, Seek, SeekFrom};
use std::iter::IntoIterator;

use crate::tdms_error::{TdmsError, TdmsErrorKind};
use crate::{ObjectMap, ReadPair, TdmsMap};
use byteorder::*;
use log::debug;
use num_derive::FromPrimitive;
use num_enum::IntoPrimitive;

#[derive(IntoPrimitive, Debug)]
#[repr(u32)]
pub enum TocProperties {
    KTocMetaData = 1 << 1,        // segment contains meta data
    KTocRawData = 1 << 3,         // segment contains raw data
    KTocDAQmxRawData = 1 << 7,    // segment contains DAQmx raw data
    KTocInterleavedData = 1 << 5, // raw data is interleaved (else continuous)
    KTocBigEndian = 1 << 6,       // all numeric values in segment are bigendian (including lead in)
    KTocNewObjList = 1 << 2, // first segment, or order has changed (is not present when channel is added)
}

#[derive(Debug)]
pub struct TocMask {
    pub flags: u32,
}

impl TocMask {
    pub fn from_flags(flags: u32) -> TocMask {
        TocMask { flags }
    }

    pub fn has_flag(&self, flag: TocProperties) -> bool {
        let flag_val: u32 = flag.into();
        (self.flags & flag_val) == flag_val
    }
}

/// The DataTypeRaw enum's values match the binary representation in
/// tdms files.
#[derive(FromPrimitive, Clone, Copy, Debug)]
#[repr(u32)]
pub enum DataTypeRaw {
    Void = 0,
    I8 = 1,
    I16 = 2,
    I32 = 3,
    I64 = 4,
    U8 = 5,
    U16 = 6,
    U32 = 7,
    U64 = 8,
    SingleFloat = 9,
    DoubleFloat = 10,
    ExtendedFloat = 11,
    SingleFloatWithUnit = 0x19,
    DoubleFloatWithUnit = 12,
    ExtendedFloatWithUnit = 13,
    TdmsString = 0x20,
    Boolean = 0x21,
    TimeStamp = 0x44,
    FixedPoint = 0x4F,
    ComplexSingleFloat = 0x0008_000c,
    ComplexDoubleFloat = 0x0010_000d,
    DAQmxRawData = 0xFFFF_FFFF,
}

impl DataTypeRaw {
    /// Convert a raw u32 value into a DataTypeRaw enum
    pub fn from_u32(raw_id: u32) -> Result<DataTypeRaw, TdmsError> {
        num::FromPrimitive::from_u32(raw_id).ok_or(TdmsError {
            kind: TdmsErrorKind::RawDataTypeNotFound,
        })
    }

    /// Returns the size of the data type in bytes.    
    pub fn size(&self) -> Result<u64, TdmsError> {
        match self {
            DataTypeRaw::Void => Ok(0),
            DataTypeRaw::I8 => Ok(1),
            DataTypeRaw::I16 => Ok(2),
            DataTypeRaw::I32 => Ok(4),
            DataTypeRaw::I64 => Ok(8),
            DataTypeRaw::U8 => Ok(1),
            DataTypeRaw::U16 => Ok(2),
            DataTypeRaw::U32 => Ok(4),
            DataTypeRaw::U64 => Ok(8),
            DataTypeRaw::SingleFloat => Ok(4),
            DataTypeRaw::DoubleFloat => Ok(8),
            DataTypeRaw::ExtendedFloat => Ok(10), // I'm guessing this is the x86 format
            DataTypeRaw::SingleFloatWithUnit => Ok(4), // Size from nptdms, not sure if correct
            DataTypeRaw::DoubleFloatWithUnit => Ok(8), // as above
            DataTypeRaw::ExtendedFloatWithUnit => Ok(10), // as above
            DataTypeRaw::Boolean => Ok(1),
            DataTypeRaw::TdmsString => Err(TdmsError {
                kind: TdmsErrorKind::StringSizeNotDefined,
            }), // size not defined
            DataTypeRaw::TimeStamp => Ok(16),
            DataTypeRaw::FixedPoint => Ok(4), // total assumption here
            DataTypeRaw::ComplexSingleFloat => Ok(8), // 2 x floats
            DataTypeRaw::ComplexDoubleFloat => Ok(16), // 2 x doubles
            DataTypeRaw::DAQmxRawData => Ok(0), // TBD
        }
    }
}

/// Wrapper for a float with unit. QUESTION: Can the genericism of this type be
/// limited to only real floats?
#[derive(Debug, Clone)]
pub struct FloatWithUnit<T> {
    repr_type: T,
    unit: String,
}

#[derive(Debug, Clone)]
pub struct TdmsTimeStamp {
    pub epoch: i64,
    pub radix: u64,
}

/// A wrapper type for data types found in tdms files
/// QUESTION: Is there a better way to allow for generic returns in "read_data" functions
#[derive(Debug, Clone)]
pub enum DataType {
    Void(()), 
    Boolean(bool), 
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Float(f32),
    Double(f64),
    //Extended(f128), Can't represent this currently
    FloatUnit(FloatWithUnit<f32>),
    DoubleUnit(FloatWithUnit<f64>),
    //ExtendedUnit(FloatWithUnit<f128>), Can't represent this currently
    TdmsString(String), // Carries a length in front
    // DaqMx(??)
    // ComplexSingle(??)
    // CompledDouble(??)
    TimeStamp(TdmsTimeStamp),
}

/// Helper function for reading string.
pub fn read_string<R: Read + Seek, O: ByteOrder>(reader: &mut R) -> Result<String, TdmsError> {
    let str_len = reader.read_u32::<O>()?;

    let mut str_raw_buf = vec![0u8; str_len as usize];
    reader.read_exact(&mut str_raw_buf)?;
    Ok(String::from_utf8(str_raw_buf)?)
}

/// Reads data into the DataType enum based on the value of DataTypeRaw.
/// The distinction exists because an enum can't have both a defined representation
/// i.e. an integer value indicating which enum value it is, and a wrapped value
pub fn read_datatype<R: Read + Seek, O: ByteOrder>(
    reader: &mut R,
    rawtype: DataTypeRaw,
) -> Result<DataType, TdmsError> {
    let dataout = match rawtype {
        DataTypeRaw::TdmsString => DataType::TdmsString(read_string::<R, O>(reader)?),
        DataTypeRaw::U8 => DataType::U8(reader.read_u8()?),
        DataTypeRaw::U16 => DataType::U16(reader.read_u16::<O>()?),
        DataTypeRaw::U32 => DataType::U32(reader.read_u32::<O>()?),
        DataTypeRaw::U64 => DataType::U64(reader.read_u64::<O>()?),
        DataTypeRaw::I8 => DataType::I8(reader.read_i8()?),
        DataTypeRaw::I16 => DataType::I16(reader.read_i16::<O>()?),
        DataTypeRaw::I32 => DataType::I32(reader.read_i32::<O>()?),
        DataTypeRaw::I64 => DataType::I64(reader.read_i64::<O>()?),
        DataTypeRaw::SingleFloat => DataType::Float(reader.read_f32::<O>()?),
        DataTypeRaw::DoubleFloat => DataType::Double(reader.read_f64::<O>()?),
        DataTypeRaw::Boolean => DataType::Boolean(match reader.read_u8()? {
            0 => false,
            _ => true,
        }),
        DataTypeRaw::TimeStamp => {
            let epoch = reader.read_i64::<O>()?;
            let radix = reader.read_u64::<O>()?;
            DataType::TimeStamp(TdmsTimeStamp { epoch, radix })
        }
        _ => DataType::Void(()), // TODO this is a dirty placeholder
    };

    Ok(dataout)
}

/// A wrapper type for vectors of data types found in tdms files
/// Previously I was using Vec<DataType> but this resulted in every
/// element coming with information about what datatype it was which
/// was un-necessary and looked gross
/// See TdmsFileHandle::read_data_vector for the point of implementation
#[derive(Debug, Clone)]
pub enum DataTypeVec {
    Void(Vec<()>), // Should nuke this somehow
    Boolean(Vec<bool>),
    I8(Vec<i8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    //Extended(Vec<f128>), Can't represent this currently
    FloatUnit(Vec<FloatWithUnit<f32>>),
    DoubleUnit(Vec<FloatWithUnit<f64>>),
    //ExtendedUnit(Vec<FloatWithUnit<f128>>), Can't represent this
    TdmsString(Vec<String>),
    // DaqMx(Vec<??>)
    // ComplexSingle(Vec<??>)
    // CompledDouble(Vec<??>)
    TimeStamp(Vec<TdmsTimeStamp>),
}

trait TdmsVector: Sized + Clone + Default {
    const BYTES: usize;
    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError>;
    fn make_vec(v: Vec<Self>) -> DataTypeVec;
}

impl TdmsVector for bool {
    const BYTES: usize = 1; 

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        let read_len = buffer.len();
        for i in 0..read_len {
            let bool_val = match reader.read_u8()? {
                0 => false,
                _ => true,
            };
            buffer[i] = bool_val;            
        }  
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::Boolean(datavec)
    }
}

impl TdmsVector for i8 {
    const BYTES: usize = 1;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_i8_into(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I8(datavec)
    }
}

impl TdmsVector for i16 {
    const BYTES: usize = 2;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_i16_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I16(datavec)
    }
}

impl TdmsVector for i32 {
    const BYTES: usize = 4;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_i32_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I32(datavec)
    }
}

impl TdmsVector for i64 {
    const BYTES: usize = 8;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_i64_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I64(datavec)
    }
}

impl TdmsVector for u8 {
    const BYTES: usize = 1;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_exact(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U8(datavec)
    }
}

impl TdmsVector for u16 {
    const BYTES: usize = 2;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_u16_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U16(datavec)
    }
}

impl TdmsVector for u32 {
    const BYTES: usize = 4;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_u32_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U32(datavec)
    }
}

impl TdmsVector for u64 {
    const BYTES: usize = 8;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_u64_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U64(datavec)
    }
}

impl TdmsVector for f32 {
    const BYTES: usize = 4;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_f32_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::Float(datavec)
    }
}

impl TdmsVector for f64 {
    const BYTES: usize = 8;

    fn read<R: Read + Seek, O: ByteOrder>(
        buffer: &mut [Self],
        reader: &mut R,
    ) -> Result<(), TdmsError> {
        reader.read_f64_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::Double(datavec)
    }
}

// impl TdmsVector for String {
//     const BYTES: usize = 8;

//     fn read<R: Read + Seek, O: ByteOrder>(
//         buffer: &mut [Self],
//         reader: &mut R,
//     ) -> Result<(), TdmsError> {
//         let mut string_lengths: Vec<u32> = Vec::new();
//         for
//         reader.read_f64_into::<O>(buffer)?;
//         Ok(())
//     }

//     fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
//         DataTypeVec::Double(datavec)
//     }
// }

fn read_into_vec<T: TdmsVector, R: Read + Seek, O: ByteOrder>(
    reader: &mut R,
    read_pairs: &Vec<ReadPair>,
    total_bytes: usize,
) -> Result<DataTypeVec, TdmsError> {
    let mut datavec: Vec<T> = vec![T::default(); total_bytes / T::BYTES];
    let mut i: usize = 0; // dummy variable to track bytes for indexing

    for pair in read_pairs {
        reader.seek(SeekFrom::Start(pair.start_index))?;
        let no_values = pair.no_bytes as usize / T::BYTES;
        if pair.interleaved {
            for j in 0..no_values {
                // exclusive range, to make sure compiler sees slice datatype
                T::read::<R, O>(&mut datavec[i + j..i + j + 1], reader)?;
                reader.seek(SeekFrom::Current(pair.stride.unwrap() as i64))?;
            }
        } else {
            T::read::<R, O>(&mut datavec[i..i + no_values], reader)?;
        }
        i += no_values;
    }
    Ok(T::make_vec(datavec))
}


pub fn read_data_vector<R: Read + Seek, O: ByteOrder>(
    object_map: &ObjectMap,
    reader: &mut R,
) -> Result<DataTypeVec, TdmsError> {
    let read_pairs = &object_map.read_map;
    let rawtype = &object_map.last_object.raw_data_type.ok_or(TdmsError {
        kind: TdmsErrorKind::ObjectHasNoRawData,
    })?;
    let total_bytes = object_map.total_bytes as usize;

    let datavec: DataTypeVec = match rawtype {
        DataTypeRaw::Void => DataTypeVec::Void(Vec::new()),
        DataTypeRaw::I8 => read_into_vec::<i8, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::I16 => read_into_vec::<i16, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::I32 => read_into_vec::<i32, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::I64 => read_into_vec::<i64, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::U8 => read_into_vec::<u8, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::U16 => read_into_vec::<u16, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::U32 => read_into_vec::<u32, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::U64 => read_into_vec::<u64, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::SingleFloat => read_into_vec::<f32, R, O>(reader, read_pairs, total_bytes)?,
        DataTypeRaw::DoubleFloat => read_into_vec::<f64, R, O>(reader, read_pairs, total_bytes)?,
        // DataTypeRaw::ExtendedFloat => {},
        // DataTypeRaw::SingleFloatWithUnit => {},
        // DataTypeRaw::DoubleFloatWithUnit => {},
        // DataTypeRaw::ExtendedFloatWithUnit => {},
        DataTypeRaw::Boolean => read_into_vec::<bool, R, O>(reader, read_pairs, total_bytes)?,
        // DataTypeRaw::TdmsString => {},
        // DataTypeRaw::TimeStamp => {},
        // DataTypeRaw::FixedPoint => {},
        // DataTypeRaw::ComplexSingleFloat => {},
        // DataTypeRaw::ComplexDoubleFloat => {},
        // DataTypeRaw::DAQmxRawData => {},
        _ => unimplemented!(),
    };
    Ok(datavec)
}

// #[derive(Debug, Clone)]
// pub struct DataTypeVec {
//     datatype: DataTypeRaw,
//     data: Vec<Box<dyn Something>>,
// }

// pub trait Something<T> {
//     fn make_native(&self) -> Vec<T>;
// }

// impl Something<u8> for DataTypeVec {
//     fn make_native(&self) -> Result<Vec<u8>, TdmsError> {
//         match self.datatype {
//             DataTypeRaw::U8 => self.data,
//             _ => TdmsErrorKind::ChannelDoesNotMatchDataType,
//         }
//     }
// }

// impl<T> Iterator for DataTypeVec<T> {
//     type Item = T;

//     fn next<T>(&mut self) -> Option<Self::Item> {}
// }

// #[derive(Debug, Clone)]
// pub struct DataTypeVec<T>(Vec<T>);

// Notes: Strings are stored concatenated in the raw data block with an array of offsets for each
// string's first character stored first in the raw data according to the Tdms Reference.
// In practise (in the Example.tdms file in this repo), this does not appear to be the case.
// For any given string channel, its raw data index is the offset to the array which in turn
// is meant to tell you where its character is. In the Example.tdms file this is not the case
// There is no preceding array of first character indices, strings are concatenated in object
// order.
