use anyhow::Result;
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

use super::{ExtMetadataBlock, ExtMetadataBlockInfo};

/// Source/mastering display color primaries
#[repr(C)]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize))]
pub struct ExtMetadataBlockLevel9 {
    pub source_primary_index: u8,
    pub source_primary_red_x: u16,
    pub source_primary_red_y: u16,
    pub source_primary_green_x: u16,
    pub source_primary_green_y: u16,
    pub source_primary_blue_x: u16,
    pub source_primary_blue_y: u16,
    pub source_primary_white_x: u16,
    pub source_primary_white_y: u16,
}

#[cfg(feature = "serde_feature")]
impl Serialize for ExtMetadataBlockLevel9 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    { 
        let mut state = serializer.serialize_struct("ExtMetadataBlockLevel9", 9)?;
        state.serialize_field("source_primary_index", &self.source_primary_index)?;

        if self.source_primary_index == 255 {
            state.serialize_field("source_primary_red_x", &self.source_primary_red_x)?;
            state.serialize_field("source_primary_red_y", &self.source_primary_white_y)?;
            state.serialize_field("source_primary_green_x", &self.source_primary_green_x)?;
            state.serialize_field("source_primary_green_y", &self.source_primary_green_y)?;
            state.serialize_field("source_primary_blue_x", &self.source_primary_blue_x)?;
            state.serialize_field("source_primary_blue_y", &self.source_primary_blue_y)?;
            state.serialize_field("source_primary_white_x", &self.source_primary_white_x)?;
            state.serialize_field("source_primary_white_y", &self.source_primary_white_y)?;
        }

        state.end()
    }
}

impl ExtMetadataBlockLevel9 {
    pub fn parse(ext_block_length: u64, reader: &mut BitVecReader) -> ExtMetadataBlock {
        let mut block = Self {
            source_primary_index: reader.get_n(8),
            ..Default::default()
        };

        if ext_block_length > 1 {
            block.source_primary_red_x = reader.get_n(16);
            block.source_primary_red_y = reader.get_n(16);
            block.source_primary_green_x = reader.get_n(16);
            block.source_primary_green_y = reader.get_n(16);
            block.source_primary_blue_x = reader.get_n(16);
            block.source_primary_blue_y = reader.get_n(16);
            block.source_primary_white_x = reader.get_n(16);
            block.source_primary_white_y = reader.get_n(16);
        };

        return ExtMetadataBlock::Level9(block);
    }

    pub fn write(&self, writer: &mut BitVecWriter) -> Result<()> {
        writer.write_n(&self.source_primary_index.to_be_bytes(), 8);

        if self.bytes_size() > 1 {
            writer.write_n(&self.source_primary_red_x.to_be_bytes(), 16);
            writer.write_n(&self.source_primary_red_y.to_be_bytes(), 16);
            writer.write_n(&self.source_primary_green_x.to_be_bytes(), 16);
            writer.write_n(&self.source_primary_green_y.to_be_bytes(), 16);
            writer.write_n(&self.source_primary_blue_x.to_be_bytes(), 16);
            writer.write_n(&self.source_primary_blue_y.to_be_bytes(), 16);
            writer.write_n(&self.source_primary_white_x.to_be_bytes(), 16);
            writer.write_n(&self.source_primary_white_y.to_be_bytes(), 16);
        }

        Ok(())
    }
}

impl ExtMetadataBlockInfo for ExtMetadataBlockLevel9 {
    fn level(&self) -> u8 {
        9
    }

    fn possible_required_bits(&self) -> Vec<u64> {
        vec![8, 136]
    }

    fn modified_fields_flag(&self) -> u64 {
        let last_field_flag = 1 << self.possible_required_bits().len() >> 2;
        let mut fields_flag = 0;
        if self.source_primary_index == 255 {
            fields_flag |= last_field_flag;
        }
        return fields_flag;
    }
}