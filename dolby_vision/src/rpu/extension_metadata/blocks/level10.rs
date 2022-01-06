use anyhow::{ensure, Result};
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

use super::{level6::MAX_PQ_LUMINANCE, ExtMetadataBlock, ExtMetadataBlockInfo};

pub const PRESET_TARGET_DISPLAYS: &[u8] = &[1, 16, 18, 21, 27, 28, 37, 38, 42, 48, 49];

/// Custom target display information
#[repr(C)]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize))]
pub struct ExtMetadataBlockLevel10 {
    pub target_display_index: u8,
    pub target_max_pq: u16,
    pub target_min_pq: u16,
    pub target_primary_index: u8,
    pub target_primary_red_x: u16,
    pub target_primary_red_y: u16,
    pub target_primary_green_x: u16,
    pub target_primary_green_y: u16,
    pub target_primary_blue_x: u16,
    pub target_primary_blue_y: u16,
    pub target_primary_white_x: u16,
    pub target_primary_white_y: u16,
}

#[cfg(feature = "serde_feature")]
impl Serialize for ExtMetadataBlockLevel10 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    { 
        let mut state = serializer.serialize_struct("ExtMetadataBlockLevel9", 12)?;
        state.serialize_field("target_display_index", &self.target_display_index)?;
        state.serialize_field("target_max_pq", &self.target_max_pq)?;
        state.serialize_field("target_min_pq", &self.target_min_pq)?;
        state.serialize_field("target_primary_index", &self.target_primary_index)?;

        if self.target_primary_index == 255 {
            state.serialize_field("target_primary_red_x", &self.target_primary_red_x)?;
            state.serialize_field("target_primary_red_y", &self.target_primary_white_y)?;
            state.serialize_field("target_primary_green_x", &self.target_primary_green_x)?;
            state.serialize_field("target_primary_green_y", &self.target_primary_green_y)?;
            state.serialize_field("target_primary_blue_x", &self.target_primary_blue_x)?;
            state.serialize_field("target_primary_blue_y", &self.target_primary_blue_y)?;
            state.serialize_field("target_primary_white_x", &self.target_primary_white_x)?;
            state.serialize_field("target_primary_white_y", &self.target_primary_white_y)?;
        }

        state.end()
    }
}

impl ExtMetadataBlockLevel10 {
    pub fn parse(ext_block_length: u64, reader: &mut BitVecReader) -> ExtMetadataBlock {
        let mut block = Self {
            target_display_index: reader.get_n(8),
            target_max_pq: reader.get_n(12),
            target_min_pq: reader.get_n(12),
            target_primary_index: reader.get_n(8),
            ..Default::default()
        };
        if ext_block_length > 5 {
            block.target_primary_red_x = reader.get_n(16);
            block.target_primary_red_y = reader.get_n(16);
            block.target_primary_green_x = reader.get_n(16);
            block.target_primary_green_y = reader.get_n(16);
            block.target_primary_blue_x = reader.get_n(16);
            block.target_primary_blue_y = reader.get_n(16);
            block.target_primary_white_x = reader.get_n(16);
            block.target_primary_white_y = reader.get_n(16);
        };

        return ExtMetadataBlock::Level10(block);
    }

    pub fn write(&self, writer: &mut BitVecWriter) -> Result<()> {
        self.validate()?;

        writer.write_n(&self.target_display_index.to_be_bytes(), 8);
        writer.write_n(&self.target_max_pq.to_be_bytes(), 12);
        writer.write_n(&self.target_min_pq.to_be_bytes(), 12);
        writer.write_n(&self.target_primary_index.to_be_bytes(), 8);

        if self.bytes_size() > 5 {
            writer.write_n(&self.target_primary_red_x.to_be_bytes(), 16);
            writer.write_n(&self.target_primary_red_y.to_be_bytes(), 16);
            writer.write_n(&self.target_primary_green_x.to_be_bytes(), 16);
            writer.write_n(&self.target_primary_green_y.to_be_bytes(), 16);
            writer.write_n(&self.target_primary_blue_x.to_be_bytes(), 16);
            writer.write_n(&self.target_primary_blue_y.to_be_bytes(), 16);
            writer.write_n(&self.target_primary_white_x.to_be_bytes(), 16);
            writer.write_n(&self.target_primary_white_y.to_be_bytes(), 16);  
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(!PRESET_TARGET_DISPLAYS.contains(&self.target_display_index));
        ensure!(self.target_max_pq <= MAX_PQ_LUMINANCE);
        ensure!(self.target_min_pq <= MAX_PQ_LUMINANCE);

        Ok(())
    }
}

impl ExtMetadataBlockInfo for ExtMetadataBlockLevel10 {
    fn level(&self) -> u8 {
        10
    }

    fn possible_required_bits(&self) -> Vec<u64> {
        vec![40, 168]
    }

    fn modified_fields_flag(&self) -> u64 {
        let last_field_flag = 1 << self.possible_required_bits().len() >> 2;
        let mut fields_flag = 0;
        if self.target_primary_index == 255 {
            fields_flag |= last_field_flag;
        }
        
        return fields_flag;
    }

    fn sort_key(&self) -> (u8, u16) {
        (self.level(), self.target_display_index as u16)
    }
}
