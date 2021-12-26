use anyhow::{ensure, Result};
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize};

use super::{level6::MAX_PQ_LUMINANCE, ExtMetadataBlock, ExtMetadataBlockInfo};

pub const PRESET_TARGET_DISPLAYS: &[u8] = &[1, 16, 18, 21, 27, 28, 37, 38, 42, 48, 49];

/// Custom target display information
#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize, Serialize))]
pub struct ExtMetadataBlockLevel10 {
    pub target_display_index: u8,
    pub target_max_pq: u16,
    pub target_min_pq: u16,
    pub target_primary_index: u8,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_red_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_red_y: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_green_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_green_y: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_blue_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_blue_y: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_white_x: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_primary_white_y: Option<u16>,
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
            block.target_primary_red_x = Some(reader.get_n(16));
            block.target_primary_red_y = Some(reader.get_n(16));
            block.target_primary_green_x = Some(reader.get_n(16));
            block.target_primary_green_y = Some(reader.get_n(16));
            block.target_primary_blue_x = Some(reader.get_n(16));
            block.target_primary_blue_y = Some(reader.get_n(16));
            block.target_primary_white_x = Some(reader.get_n(16));
            block.target_primary_white_y = Some(reader.get_n(16));
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
            writer.write_n(&self.target_primary_red_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.target_primary_red_y.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.target_primary_green_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.target_primary_green_y.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.target_primary_blue_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.target_primary_blue_y.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.target_primary_white_x.unwrap().to_be_bytes(), 16);
            writer.write_n(&self.target_primary_white_y.unwrap().to_be_bytes(), 16);  
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

    fn bytes_size(&self) -> u64 {
        match self.required_bits() {
            168 => 21,
            _ => 5,
        }
    }

    fn required_bits(&self) -> u64 {
        let mut bits = 168;
        if self.target_primary_red_x.is_none()
        || self.target_primary_red_y.is_none()
        || self.target_primary_green_x.is_none()
        || self.target_primary_green_y.is_none()
        || self.target_primary_blue_x.is_none()
        || self.target_primary_blue_y.is_none()
        || self.target_primary_white_x.is_none()
        || self.target_primary_white_y.is_none() {
            bits -= 128;
        }
        return bits;
    }

    fn possible_bytes_size(&self) -> Vec<u64> {
        vec![5, 21]
    }

    fn possible_required_bits(&self) -> Vec<u64> {
        vec![40, 168]
    }

    fn possible_bits_size(&self) -> Vec<u64> {
        vec![40, 168]
    }
}

impl Default for ExtMetadataBlockLevel10 {
    fn default() -> Self {
        Self {
            target_display_index: 48,
            target_max_pq: 3079,
            target_min_pq: 0,
            target_primary_index: 0,
            target_primary_red_x: None,
            target_primary_red_y: None,
            target_primary_green_x: None,
            target_primary_green_y: None,
            target_primary_blue_x: None,
            target_primary_blue_y: None,
            target_primary_white_x: None,
            target_primary_white_y: None,
        }
    }
}
