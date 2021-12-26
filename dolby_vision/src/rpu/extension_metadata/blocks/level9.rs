use anyhow::Result;
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize};

use super::{ExtMetadataBlock, ExtMetadataBlockInfo};

/// Source/mastering display color primaries
#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize, Serialize))]
pub struct ExtMetadataBlockLevel9 {
    pub source_primary_index: u8,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_red_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_red_y: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_green_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_green_y: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_blue_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_blue_y: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_white_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub source_primary_white_y: Option<u16>,
}

impl ExtMetadataBlockLevel9 {
    pub fn parse(ext_block_length: u64, reader: &mut BitVecReader) -> ExtMetadataBlock {
        let mut block = Self {
            source_primary_index: reader.get_n(8),
            ..Default::default()
        };

        if ext_block_length > 1 {
            block.source_primary_red_x = Some(reader.get_n(16));
            block.source_primary_red_y = Some(reader.get_n(16));
            block.source_primary_green_x = Some(reader.get_n(16));
            block.source_primary_green_y = Some(reader.get_n(16));
            block.source_primary_blue_x = Some(reader.get_n(16));
            block.source_primary_blue_y = Some(reader.get_n(16));
            block.source_primary_white_x = Some(reader.get_n(16));
            block.source_primary_white_y = Some(reader.get_n(16));
        };

        return ExtMetadataBlock::Level9(block);
    }

    pub fn write(&self, writer: &mut BitVecWriter) -> Result<()> {
        writer.write_n(&self.source_primary_index.to_be_bytes(), 8);

        if self.bytes_size() > 1 {
            writer.write_n(&self.source_primary_red_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.source_primary_red_y.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.source_primary_green_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.source_primary_green_y.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.source_primary_blue_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.source_primary_blue_y.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.source_primary_white_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.source_primary_white_y.unwrap().to_be_bytes(), 16);
        }

        Ok(())
    }
}

impl ExtMetadataBlockInfo for ExtMetadataBlockLevel9 {
    fn level(&self) -> u8 {
        9
    }

    fn bytes_size(&self) -> u64 {
        match self.required_bits() {
            136 => 17,
            _ => 1,
        }
    }

    fn required_bits(&self) -> u64 {
        let mut bits = 136;
        if self.source_primary_red_x.is_none()
        || self.source_primary_red_y.is_none()
        || self.source_primary_green_x.is_none()
        || self.source_primary_green_y.is_none()
        || self.source_primary_blue_x.is_none()
        || self.source_primary_blue_y.is_none()
        || self.source_primary_white_x.is_none()
        || self.source_primary_white_y.is_none()
        {
            bits -= 128;
        }
        return bits;
    }

    fn possible_bytes_size(&self) -> Vec<u64> {
        vec![1, 17]
    }

    fn possible_required_bits(&self) -> Vec<u64> {
        vec![8, 136]
    }

    fn possible_bits_size(&self) -> Vec<u64> {
        vec![8, 136]
    }
}

/// Default mastering display primaries: P3, D65
impl Default for ExtMetadataBlockLevel9 {
    fn default() -> Self {
        Self {
            source_primary_index: 0,
            source_primary_red_x: None,
            source_primary_red_y: None,
            source_primary_green_x: None,
            source_primary_green_y: None,
            source_primary_blue_x: None,
            source_primary_blue_y: None,
            source_primary_white_x: None,
            source_primary_white_y: None,
        }
    }
}