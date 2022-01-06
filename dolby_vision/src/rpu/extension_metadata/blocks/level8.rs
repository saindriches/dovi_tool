use anyhow::{ensure, Result};
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

use super::{ExtMetadataBlock, ExtMetadataBlockInfo, MAX_12_BIT_VALUE};

/// Creative intent trim passes per target display peak brightness
/// For CM v4.0, L8 metadata only is present and used to compute L2
#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize))]
pub struct ExtMetadataBlockLevel8 {
    pub target_display_index: u8,
    pub trim_slope: u16,
    pub trim_offset: u16,
    pub trim_power: u16,
    pub trim_chroma_weight: u16,
    pub trim_saturation_gain: u16,
    pub ms_weight: u16,
    pub target_mid_contrast: u16,
    pub clip_trim: u16,
    pub saturation_vector_field0: u8,
    pub saturation_vector_field1: u8,
    pub saturation_vector_field2: u8,
    pub saturation_vector_field3: u8,
    pub saturation_vector_field4: u8,
    pub saturation_vector_field5: u8,
    pub hue_vector_field0: u8,
    pub hue_vector_field1: u8,
    pub hue_vector_field2: u8,
    pub hue_vector_field3: u8,
    pub hue_vector_field4: u8,
    pub hue_vector_field5: u8,
}

#[cfg(feature = "serde_feature")]
impl Serialize for ExtMetadataBlockLevel8 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    { 
        let mut state = serializer.serialize_struct("ExtMetadataBlockLevel8", 21)?;
        state.serialize_field("target_display_index", &self.target_display_index)?;
        state.serialize_field("trim_slope", &self.trim_slope)?;
        state.serialize_field("trim_offset", &self.trim_offset)?;
        state.serialize_field("trim_power", &self.trim_power)?;
        state.serialize_field("trim_chroma_weight", &self.trim_chroma_weight)?;
        state.serialize_field("trim_saturation_gain", &self.trim_saturation_gain)?;
        state.serialize_field("ms_weight", &self.ms_weight)?;

        let length = self.bytes_size();
        if length > 10 {
            state.serialize_field("target_mid_contrast", &self.target_mid_contrast)?;
        }
        if length > 12 {
            state.serialize_field("clip_trim", &self.clip_trim)?;
        }
        if length > 13 {
            state.serialize_field("saturation_vector_field0", &self.saturation_vector_field0)?;
            state.serialize_field("saturation_vector_field1", &self.saturation_vector_field1)?;
            state.serialize_field("saturation_vector_field2", &self.saturation_vector_field2)?;
            state.serialize_field("saturation_vector_field3", &self.saturation_vector_field3)?;
            state.serialize_field("saturation_vector_field4", &self.saturation_vector_field4)?;
            state.serialize_field("saturation_vector_field5", &self.saturation_vector_field5)?;
        }
        if length > 19 {
            state.serialize_field("hue_vector_field0", &self.hue_vector_field0)?;
            state.serialize_field("hue_vector_field1", &self.hue_vector_field1)?;
            state.serialize_field("hue_vector_field2", &self.hue_vector_field2)?;
            state.serialize_field("hue_vector_field3", &self.hue_vector_field3)?;
            state.serialize_field("hue_vector_field4", &self.hue_vector_field4)?;
            state.serialize_field("hue_vector_field5", &self.hue_vector_field5)?;
        }
        
        state.end()
    }
}

impl ExtMetadataBlockLevel8 {
    pub fn parse(ext_block_length: u64, reader: &mut BitVecReader) -> ExtMetadataBlock {
        let mut block = Self {
            target_display_index: reader.get_n(8),
            trim_slope: reader.get_n(12),
            trim_offset: reader.get_n(12),
            trim_power: reader.get_n(12),
            trim_chroma_weight: reader.get_n(12),
            trim_saturation_gain: reader.get_n(12),
            ms_weight: reader.get_n(12),
            ..Default::default()
        };

        if ext_block_length > 10 {
            block.target_mid_contrast = reader.get_n(12);
        };
        if ext_block_length > 12 {
            block.clip_trim = reader.get_n(12);
        };
        if ext_block_length > 13 {
            block.saturation_vector_field0 = reader.get_n(8);
            block.saturation_vector_field1 = reader.get_n(8);
            block.saturation_vector_field2 = reader.get_n(8);
            block.saturation_vector_field3 = reader.get_n(8);
            block.saturation_vector_field4 = reader.get_n(8);
            block.saturation_vector_field5 = reader.get_n(8);
        };
        if ext_block_length > 19 {
            block.hue_vector_field0 = reader.get_n(8);
            block.hue_vector_field1 = reader.get_n(8);
            block.hue_vector_field2 = reader.get_n(8);
            block.hue_vector_field3 = reader.get_n(8);
            block.hue_vector_field4 = reader.get_n(8);
            block.hue_vector_field5 = reader.get_n(8);
        };

        return ExtMetadataBlock::Level8(block);
    }

    pub fn write(&self, writer: &mut BitVecWriter) -> Result<()> {
        self.validate()?;
        let length = self.bytes_size();

        writer.write_n(&self.target_display_index.to_be_bytes(), 8);
        writer.write_n(&self.trim_slope.to_be_bytes(), 12);
        writer.write_n(&self.trim_offset.to_be_bytes(), 12);
        writer.write_n(&self.trim_power.to_be_bytes(), 12);
        writer.write_n(&self.trim_chroma_weight.to_be_bytes(), 12);
        writer.write_n(&self.trim_saturation_gain.to_be_bytes(), 12);
        writer.write_n(&self.ms_weight.to_be_bytes(), 12);

        // Write default values when the fields can not be omitted
        if length > 10 {
            writer.write_n(&self.target_mid_contrast.to_be_bytes(), 12);
        };
        if length > 12 {
            writer.write_n(&self.clip_trim.to_be_bytes(), 12);
        };
        if length > 13 {
            writer.write_n(&self.saturation_vector_field0.to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field1.to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field2.to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field3.to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field4.to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field5.to_be_bytes(), 8);
        };
        if length > 19 {
            writer.write_n(&self.hue_vector_field0.to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field1.to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field2.to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field3.to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field4.to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field5.to_be_bytes(), 8);
        };

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.trim_slope <= MAX_12_BIT_VALUE);
        ensure!(self.trim_offset <= MAX_12_BIT_VALUE);
        ensure!(self.trim_power <= MAX_12_BIT_VALUE);
        ensure!(self.trim_chroma_weight <= MAX_12_BIT_VALUE);
        ensure!(self.trim_saturation_gain <= MAX_12_BIT_VALUE);
        ensure!(self.ms_weight <= MAX_12_BIT_VALUE);
        ensure!(self.target_mid_contrast <= MAX_12_BIT_VALUE);
        ensure!(self.clip_trim <= MAX_12_BIT_VALUE);

        Ok(())
    }
}

impl ExtMetadataBlockInfo for ExtMetadataBlockLevel8 {
    fn level(&self) -> u8 {
        8
    }

    fn possible_required_bits(&self) -> Vec<u64> {
        vec![80, 92, 104, 152, 200]
    }

    fn modified_fields_flag(&self) -> u64 {
        let last_field_flag = 1 << self.possible_required_bits().len() >> 2;
        let mut index = 0;
        let mut fields_flag = 0;
        let default: Self = Default::default();
        if self.hue_vector_field0 != default.hue_vector_field0
        || self.hue_vector_field1 != default.hue_vector_field1
        || self.hue_vector_field2 != default.hue_vector_field2
        || self.hue_vector_field3 != default.hue_vector_field3
        || self.hue_vector_field4 != default.hue_vector_field4
        || self.hue_vector_field5 != default.hue_vector_field5
        { 
            fields_flag |= last_field_flag >> index;
        };
        index += 1;
        if self.saturation_vector_field0 != default.saturation_vector_field0
        || self.saturation_vector_field1 != default.saturation_vector_field1
        || self.saturation_vector_field2 != default.saturation_vector_field2
        || self.saturation_vector_field3 != default.saturation_vector_field3
        || self.saturation_vector_field4 != default.saturation_vector_field4
        || self.saturation_vector_field5 != default.saturation_vector_field5
        {
            fields_flag |= last_field_flag >> index;
        };
        index += 1;
        if self.clip_trim != default.clip_trim {
            fields_flag |= last_field_flag >> index;
        };
        index += 1;
        if self.target_mid_contrast != default.target_mid_contrast {
            fields_flag |= last_field_flag >> index;
        };
        
        return fields_flag;
    }

    fn sort_key(&self) -> (u8, u16) {
        (self.level(), self.target_display_index as u16)
    }
}

impl Default for ExtMetadataBlockLevel8 {
    fn default() -> Self {
        Self {
            target_display_index: 1,
            trim_slope: 2048,
            trim_offset: 2048,
            trim_power: 2048,
            trim_chroma_weight: 2048,
            trim_saturation_gain: 2048,
            ms_weight: 2048,
            target_mid_contrast: 2048,
            clip_trim: 2048,
            saturation_vector_field0: 128,
            saturation_vector_field1: 128,
            saturation_vector_field2: 128,
            saturation_vector_field3: 128,
            saturation_vector_field4: 128,
            saturation_vector_field5: 128,
            hue_vector_field0: 128,
            hue_vector_field1: 128,
            hue_vector_field2: 128,
            hue_vector_field3: 128,
            hue_vector_field4: 128,
            hue_vector_field5: 128,
        }
    }
}
