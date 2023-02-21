use std::convert::TryFrom;
use std::io::{Read, Seek, SeekFrom};

use crate::tdms_error::{Result, TdmsError};
use crate::timestamps::TimeStamp;
use crate::{ObjectMap, ReadPair};
use byteorder::*;
use log::debug;
use num_derive::FromPrimitive;
use num_enum::IntoPrimitive;

/// An enum of bit flags indicating various data configuration options at the
/// segment level.
#[derive(IntoPrimitive, Debug)]
#[repr(u32)]
pub enum TocProperties {
    /// segment contains meta data
    KTocMetaData = 1 << 1,
    /// segment contains raw data
    KTocRawData = 1 << 3,
    /// segment contains DAQmx raw data    
    KTocDAQmxRawData = 1 << 7,
    /// raw data is interleaved (else continuous)
    KTocInterleavedData = 1 << 5,
    /// all numeric values in segment are bigendian (including lead in)
    KTocBigEndian = 1 << 6,
    /// first segment, or order has changed (is not present when channel is added)
    KTocNewObjList = 1 << 2,
}

#[derive(Debug)]
pub struct TocMask {
    pub flags: u32,
}

impl TocMask {
    /// Convert a u32 into a ToCMask struct
    pub fn from_flags(flags: u32) -> TocMask {
        TocMask { flags }
    }

    /// Check if a ToCMask has a given flag
    pub fn has_flag(&self, flag: TocProperties) -> bool {
        let flag_val: u32 = flag.into();
        (self.flags & flag_val) == flag_val
    }
}

/// The DataTypeRaw enum's values match the binary representation of that
/// type in tdms files.
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
    pub fn from_u32(raw_id: u32) -> Result<DataTypeRaw> {
        num::FromPrimitive::from_u32(raw_id).ok_or(TdmsError::RawDataTypeNotFound)
    }

    /// Returns the size of the data type in bytes.    
    pub fn size(&self) -> Result<u64> {
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
            DataTypeRaw::SingleFloatWithUnit => Ok(4),
            DataTypeRaw::DoubleFloatWithUnit => Ok(8),
            DataTypeRaw::ExtendedFloatWithUnit => Ok(10),
            DataTypeRaw::Boolean => Ok(1),
            DataTypeRaw::TdmsString => Err(TdmsError::StringSizeNotDefined),
            DataTypeRaw::TimeStamp => Ok(16),
            DataTypeRaw::FixedPoint => Ok(4), // total assumption here
            DataTypeRaw::ComplexSingleFloat => Ok(8), // 2 x floats
            DataTypeRaw::ComplexDoubleFloat => Ok(16), // 2 x doubles
            DataTypeRaw::DAQmxRawData => Ok(0), // TBD
        }
    }
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
    // Extended(f128), // Can't represent this currently
    // FloatUnit(f32), // These don't exist, they're a normal f32 paired with a property
    // DoubleUnit(f64), // as above
    //ExtendedUnit(FloatWithUnit<f128>), // Can't represent this currently
    TdmsString(String),
    DaqMx(f64), // I think these don't exist, it's a normal double with properties
    // ComplexSingle(??)
    // CompledDouble(??)
    TimeStamp(TimeStamp),
}

/// Helper function for reading a string from file.
pub fn read_string<R: Read + Seek, O: ByteOrder>(reader: &mut R) -> Result<String> {
    let str_len = reader.read_u32::<O>()?;

    let mut str_raw_buf = vec![0u8; str_len as usize];
    reader.read_exact(&mut str_raw_buf)?;
    Ok(String::from_utf8(str_raw_buf)?)
}

/// Reads data into the DataType enum based on the value of DataTypeRaw.
pub fn read_datatype<R: Read + Seek, O: ByteOrder>(
    reader: &mut R,
    rawtype: DataTypeRaw,
) -> Result<DataType> {
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
        DataTypeRaw::Boolean => DataType::Boolean(!matches!(reader.read_u8()?, 0)),
        DataTypeRaw::TimeStamp => {
            let epoch = reader.read_i64::<O>()?;
            let radix = reader.read_u64::<O>()?;
            DataType::TimeStamp(TimeStamp { epoch, radix })
        }
        DataTypeRaw::DAQmxRawData => DataType::DaqMx(reader.read_f64::<O>()?),
        _ => unimplemented!(),
    };

    Ok(dataout)
}

/// A wrapper type for vectors of data types found in tdms files
#[derive(Debug, Clone)]
pub enum DataTypeVec {
    Void(Vec<()>),
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
    // Extended(Vec<f128>),     // Can't represent this currently
    // FloatUnit(Vec<f32>),     // Don't exist as distinct types in files
    // DoubleUnit(Vec<f64>),    // Don't exist as distinct types in files
    // ExtendedUnit(Vec<FloatWithUnit<f128>>), Can't represent this
    TdmsString(Vec<String>),
    // DaqMx(Vec<??>),          // Don't exist as distinct types in files
    // ComplexSingle(Vec<??>)
    // CompledDouble(Vec<??>)
    TimeStamp(Vec<TimeStamp>),
}

/// Defines functionality required to read and construct a vector of Tdms
/// data types
trait TdmsVector: Sized + Clone + Default {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()>;

    fn make_vec(v: Vec<Self>) -> DataTypeVec;
}

impl TdmsVector for bool {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        for item in buffer.iter_mut() {
            *item = !matches!(reader.read_u8()?, 0);
        }
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::Boolean(datavec)
    }
}

impl TdmsVector for i8 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_i8_into(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I8(datavec)
    }
}

impl TdmsVector for i16 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_i16_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I16(datavec)
    }
}

impl TdmsVector for i32 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_i32_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I32(datavec)
    }
}

impl TdmsVector for i64 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_i64_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::I64(datavec)
    }
}

impl TdmsVector for u8 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_exact(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U8(datavec)
    }
}

impl TdmsVector for u16 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_u16_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U16(datavec)
    }
}

impl TdmsVector for u32 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_u32_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U32(datavec)
    }
}

impl TdmsVector for u64 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_u64_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::U64(datavec)
    }
}

impl TdmsVector for f32 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_f32_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::Float(datavec)
    }
}

impl TdmsVector for f64 {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        reader.read_f64_into::<O>(buffer)?;
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::Double(datavec)
    }
}

impl TdmsVector for String {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        let mut string_lengths: Vec<u32> = Vec::new();
        for _ in 0..buffer.len() {
            string_lengths.push(reader.read_u32::<O>()?);
        }

        for i in 0..buffer.len() {
            let mut str_raw_buf = if i == 0 {
                vec![0u8; string_lengths[i] as usize]
            } else {
                vec![0u8; (string_lengths[i] - string_lengths[i - 1]) as usize]
            };
            reader.read_exact(&mut str_raw_buf)?;
            buffer[i] = String::from_utf8(str_raw_buf)?;
        }
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::TdmsString(datavec)
    }
}

impl TdmsVector for TimeStamp {
    fn read<R: Read + Seek, O: ByteOrder>(buffer: &mut [Self], reader: &mut R) -> Result<()> {
        for item in buffer.iter_mut() {
            let epoch = reader.read_i64::<O>()?;
            let radix = reader.read_u64::<O>()?;
            *item = TimeStamp { epoch, radix };
        }
        Ok(())
    }

    fn make_vec(datavec: Vec<Self>) -> DataTypeVec {
        DataTypeVec::TimeStamp(datavec)
    }
}

/// A generic function for reading different data types into a DataTypeVec enum
/// dispatches to implementations according to type
fn read_into_vec<T: TdmsVector, R: Read + Seek, O: ByteOrder>(
    reader: &mut R,
    read_pairs: &[ReadPair],
    total_values: usize,
) -> Result<DataTypeVec> {
    let mut datavec: Vec<T> = vec![T::default(); total_values];
    let mut i: usize = 0; // dummy variable to track values for indexing

    for pair in read_pairs {
        reader.seek(SeekFrom::Start(pair.start_index))?;
        let no_values = pair.no_values as usize; // Maybe suspect for the interleaved comp
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

/// Read a vector of a given tdms data type associated with an object,
///  depending on the raw data type recorded for that object
pub fn read_data_vector<R: Read + Seek, O: ByteOrder>(
    object_map: &ObjectMap,
    reader: &mut R,
) -> Result<DataTypeVec> {
    let read_pairs = &object_map.read_map;
    let rawtype = &object_map
        .last_object
        .raw_data_type
        .ok_or(TdmsError::ObjectHasNoRawData)?;
    let total_values = object_map.total_values;
    debug!("Map total values: {}", total_values);

    let datavec: DataTypeVec = match rawtype {
        DataTypeRaw::Void => DataTypeVec::Void(Vec::new()),
        DataTypeRaw::I8 => read_into_vec::<i8, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::I16 => read_into_vec::<i16, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::I32 => read_into_vec::<i32, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::I64 => read_into_vec::<i64, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::U8 => read_into_vec::<u8, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::U16 => read_into_vec::<u16, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::U32 => read_into_vec::<u32, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::U64 => read_into_vec::<u64, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::SingleFloat => read_into_vec::<f32, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::DoubleFloat => read_into_vec::<f64, R, O>(reader, read_pairs, total_values)?,
        // DataTypeRaw::ExtendedFloat => {},
        // DataTypeRaw::SingleFloatWithUnit => {},
        // DataTypeRaw::DoubleFloatWithUnit => {},
        // DataTypeRaw::ExtendedFloatWithUnit => {},
        DataTypeRaw::Boolean => read_into_vec::<bool, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::TdmsString => read_into_vec::<String, R, O>(reader, read_pairs, total_values)?,
        DataTypeRaw::TimeStamp => {
            read_into_vec::<TimeStamp, R, O>(reader, read_pairs, total_values)?
        }
        // DataTypeRaw::FixedPoint => {},
        // DataTypeRaw::ComplexSingleFloat => {},
        // DataTypeRaw::ComplexDoubleFloat => {},
        // DataTypeRaw::DAQmxRawData => {},
        _ => unimplemented!(),
    };
    Ok(datavec)
}

impl TryFrom<DataTypeVec> for Vec<f64> {
    type Error = TdmsError;

    fn try_from(in_vec: DataTypeVec) -> Result<Self> {
        match in_vec {
            //Void(datavec) => ,
            DataTypeVec::Boolean(datavec) => {
                let out_vec: Vec<f64> =
                    datavec.iter().map(|x| if *x { 1.0 } else { 0.0 }).collect();
                Ok(out_vec)
            }
            DataTypeVec::I8(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::I16(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::I32(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::I64(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::U8(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::U16(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::U32(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::U64(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::Float(datavec) => {
                let mut out_vec: Vec<f64> = vec![0.0; datavec.len()];
                for (i, elem) in out_vec.iter_mut().enumerate() {
                    *elem = datavec[i] as f64;
                }
                Ok(out_vec)
            }
            DataTypeVec::Double(datavec) => Ok(datavec),
            // Extended(Vec<f128>),     // Can't represent this currently
            // FloatUnit(Vec<f32>),     // Don't exist as distinct types in files
            // DoubleUnit(Vec<f64>),    // Don't exist as distinct types in files
            // ExtendedUnit(Vec<FloatWithUnit<f128>>), Can't represent this
            // TdmsString(Vec<String>),
            // DaqMx(Vec<??>),          // Don't exist as distinct types in files
            // ComplexSingle(Vec<??>)
            // CompledDouble(Vec<??>)
            // TimeStamp(Vec<TimeStamp>),
            _ => unimplemented!(),
        }
    }
}
