use crate::tdms_datatypes::*;
use crate::tdms_error::*;
use byteorder::*;
use indexmap::IndexMap;
use log::debug;
use std::fmt;
use std::io::{Read, Seek};

#[derive(Debug, Clone, Default)]
pub struct TdmsObject {
    pub object_path: String,
    /// The length in bytes of the indexing info for raw data, including the length of this field. Should always be 20 (defined length) or 28 (variable length)
    pub index_info_len: u32,
    pub raw_data_type: Option<DataTypeRaw>,
    pub raw_data_dim: Option<u32>,
    pub no_raw_vals: Option<u64>,
    /// of raw data in bytes, appears in file for variable length types (String) only, computed otherwise
    pub no_bytes: u64,
    pub no_properties: u32,
    pub daqmx_info: Option<DAQMxInfo>,
    pub properties: IndexMap<String, ObjectProperty>,
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
    /// Performs the sequence of reads required to establish the size of raw data for an object
    pub(crate) fn read_sizeinfo<R: Read + Seek, O: ByteOrder>(
        &mut self,
        reader: &mut R,
    ) -> Result<&mut Self> {
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

    /// Performs the sequence of reads to establish Daqmx Info
    pub(crate) fn read_daqmxinfo<R: Read + Seek, O: ByteOrder>(
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
    pub(crate) fn update_properties<R: Read + Seek, O: ByteOrder>(
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
    value: DataType,
}

impl fmt::Display for ObjectProperty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Property name: {}", self.prop_name)?;
        writeln!(f, "Property datatype: {:?}", self.data_type)?;
        writeln!(f, "Property val: {:?}", self.value)?;
        Ok(())
    }
}

impl ObjectProperty {
    /// Instantiate a property and read into it.
    pub(crate) fn read_property<R: Read + Seek, O: ByteOrder>(
        reader: &mut R,
    ) -> Result<ObjectProperty> {
        let prop_name = read_string::<R, O>(reader)?;
        let data_type = DataTypeRaw::from_u32(reader.read_u32::<O>()?)?;
        let value = read_datatype::<R, O>(reader, data_type)?;
        Ok(ObjectProperty {
            prop_name,
            data_type,
            value,
        })
    }
}
