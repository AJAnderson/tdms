use num_derive::FromPrimitive;
use std::fmt;

#[repr(u32)]
pub enum TocProperties {
    KTocMetaData = 1 << 1,
    KTocRawData = 1 << 3,
    KTocDAQmxRawData = 1 << 7,
    KTocInterleavedData = 1 << 5,
    KTocBigEndian = 1 << 6,
    KTocNewObjList = 1 << 2,
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

/// Wrapper for a float with unit. QUESTION: Can the genericism of this type be
/// limited to only real floats?
#[derive(Debug)]
pub struct FloatWithUnit<T> {
    repr_type: T,
    unit: String,
}

/// A wrapper type for data types found in tdms files
/// QUESTION: Is there a better way to allow for generic returns in "read_data" functions
#[derive(Debug)]
pub enum DataType {
    Void(()),      // Should nuke this somehow
    Boolean(bool), // nptdms uses 1 byte, I'm not sure this is correct as LV internal representation is 32 bits for a bool
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
                        // TimeStamp(??)
}

/// A wrapper type for vectors of data types found in tdms files
/// Previously I was using Vec<DataType> but this resulted in every
/// element coming with information about what datatype it was which
/// was un-necessary and looked gross
/// See TdmsFileHandle::read_data_vector for the point of implementation
#[derive(Debug)]
pub enum DataTypeVec {
    Void(Vec<()>),      // Should nuke this somehow
    Boolean(Vec<bool>), // nptdms uses 1 byte, I'm not sure this is correct as LV internal representation is 32 bits for a bool
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
    // TimeStamp(Vec<??>)
}

// Notes: Strings are stored concatenated in the raw data block with an array of offsets for each
// string's first character stored first in the raw data according to the Tdms Reference.
// In practise (in the Example.tdms file in this repo), this does not appear to be the case.
// For any given string channel, it's raw data index is the offset to the array which in turn
// is meant to tell you where it's character is. In the Example.tdms file this is not the case
// There is no preceding array of first character indices, strings are concatenated in object
// order.
