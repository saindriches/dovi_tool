use anyhow::{ensure, Result};
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize};

use super::{ExtMetadataBlock, ExtMetadataBlockInfo, MAX_12_BIT_VALUE};

/// Creative intent trim passes per target display peak brightness
/// For CM v4.0, L8 metadata only is present and used to compute L2
#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize, Serialize))]
pub struct ExtMetadataBlockLevel8 {
    pub target_display_index: u8,
    pub trim_slope: u16,
    pub trim_offset: u16,
    pub trim_power: u16,
    pub trim_chroma_weight: u16,
    pub trim_saturation_gain: u16,
    pub ms_weight: u16,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub target_mid_contrast: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub clip_trim: Option<u16>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub saturation_vector_field0: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub saturation_vector_field1: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub saturation_vector_field2: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub saturation_vector_field3: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub saturation_vector_field4: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub saturation_vector_field5: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub hue_vector_field0: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub hue_vector_field1: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub hue_vector_field2: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub hue_vector_field3: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub hue_vector_field4: Option<u8>,
    #[cfg_attr(feature = "serde_feature", serde(skip_serializing_if = "Option::is_none"))]
    pub hue_vector_field5: Option<u8>,
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
            block.target_mid_contrast = Some(reader.get_n(12));
        };
        if ext_block_length > 12 {
            block.clip_trim = Some(reader.get_n(12));
        };
        if ext_block_length > 13 {
            block.saturation_vector_field0 = Some(reader.get_n(8));
            block.saturation_vector_field1 = Some(reader.get_n(8));
            block.saturation_vector_field2 = Some(reader.get_n(8));
            block.saturation_vector_field3 = Some(reader.get_n(8));
            block.saturation_vector_field4 = Some(reader.get_n(8));
            block.saturation_vector_field5 = Some(reader.get_n(8));
        };
        if ext_block_length > 19 {
            block.hue_vector_field0 = Some(reader.get_n(8));
            block.hue_vector_field1 = Some(reader.get_n(8));
            block.hue_vector_field2 = Some(reader.get_n(8));
            block.hue_vector_field3 = Some(reader.get_n(8));
            block.hue_vector_field4 = Some(reader.get_n(8));
            block.hue_vector_field5 = Some(reader.get_n(8));
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
        if length > 10 {
            writer.write_n(&self.target_mid_contrast.unwrap().to_be_bytes(), 12);
        };
        if length > 12 {
            writer.write_n(&self.clip_trim.unwrap().to_be_bytes(), 12);
        };
        if length > 13 {
            writer.write_n(&self.saturation_vector_field0.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field1.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field2.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field3.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field4.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.saturation_vector_field5.unwrap().to_be_bytes(), 8);
        };
        if length > 19 {
            writer.write_n(&self.hue_vector_field0.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field1.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field2.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field3.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field4.unwrap().to_be_bytes(), 8);
            writer.write_n(&self.hue_vector_field5.unwrap().to_be_bytes(), 8);
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.trim_slope <= MAX_12_BIT_VALUE);
        ensure!(self.trim_offset <= MAX_12_BIT_VALUE);
        ensure!(self.trim_power <= MAX_12_BIT_VALUE);
        ensure!(self.trim_chroma_weight <= MAX_12_BIT_VALUE);
        ensure!(self.trim_saturation_gain <= MAX_12_BIT_VALUE);
        ensure!(self.ms_weight <= MAX_12_BIT_VALUE);
        if self.target_mid_contrast.is_some() {
            ensure!(self.target_mid_contrast.unwrap() <= MAX_12_BIT_VALUE)
        };
        if self.target_mid_contrast.is_some() {
            ensure!(self.clip_trim.unwrap() <= MAX_12_BIT_VALUE)
        };

        Ok(())
    }
}

impl ExtMetadataBlockInfo for ExtMetadataBlockLevel8 {
    fn level(&self) -> u8 {
        8
    }

    fn bytes_size(&self) -> u64 {
        match self.required_bits() {
            200 => 25,
            152 => 19,
            104 => 13,
            92 => 12,
            _ => 10,
        }
    }

    fn required_bits(&self) -> u64 {
        let mut bit_flag: u8 = 0b1111;
        if self.hue_vector_field0.is_none()
        || self.hue_vector_field1.is_none()
        || self.hue_vector_field2.is_none()
        || self.hue_vector_field3.is_none()
        || self.hue_vector_field4.is_none()
        || self.hue_vector_field5.is_none() 
        { 
            bit_flag &= !0b1000;
        };
        if self.saturation_vector_field0.is_none()
        || self.saturation_vector_field1.is_none()
        || self.saturation_vector_field2.is_none()
        || self.saturation_vector_field3.is_none()
        || self.saturation_vector_field4.is_none()
        || self.saturation_vector_field5.is_none()
        {
            bit_flag &= !0b0100;
        };
        if self.clip_trim.is_none() {
            bit_flag &= !0b0010
        };
        if self.target_mid_contrast.is_none() {
            bit_flag &= !0b0001
        };

        let mut bits = 200;
        if bit_flag & 0b1000 == 0 {bits -= 48};
        if bit_flag & 0b0100 == 0 {bits -= 48};
        if bit_flag & 0b0010 == 0 {bits -= 12};
        if bit_flag & 0b0001 == 0 {bits -= 12};

        return bits;
    }

    fn possible_bytes_size(&self) -> Vec<u64> {
        vec![10, 12, 13, 19, 25]
    }

    fn possible_required_bits(&self) -> Vec<u64> {
        vec![80, 92, 104, 152, 200]
    }

    fn possible_bits_size(&self) -> Vec<u64> {
        vec![80, 96, 104, 152, 200]
    }

    fn sort_key(&self) -> (u8, u16) {
        (self.level(), self.target_display_index as u16)
    }
}

/// Target display: 1000-nit, P3, D65, ST.2084, Full (HOME)
impl Default for ExtMetadataBlockLevel8 {
    fn default() -> Self {
        Self {
            target_display_index: 48,
            trim_slope: 2048,
            trim_offset: 2048,
            trim_power: 2048,
            trim_chroma_weight: 2048,
            trim_saturation_gain: 2048,
            ms_weight: 2048,
            target_mid_contrast: None,
            clip_trim: None,
            saturation_vector_field0: None,
            saturation_vector_field1: None,
            saturation_vector_field2: None,
            saturation_vector_field3: None,
            saturation_vector_field4: None,
            saturation_vector_field5: None,
            hue_vector_field0: None,
            hue_vector_field1: None,
            hue_vector_field2: None,
            hue_vector_field3: None,
            hue_vector_field4: None,
            hue_vector_field5: None,
        }
    }
}
