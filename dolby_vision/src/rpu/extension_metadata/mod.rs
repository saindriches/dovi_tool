use anyhow::{ensure, Result};
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize};

pub mod blocks;
pub mod cmv29;
pub mod cmv40;

pub use cmv29::CmV29DmData;
pub use cmv40::CmV40DmData;

use blocks::ExtMetadataBlock;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde_feature", serde(untagged))]
pub enum DmData {
    V29(CmV29DmData),
    V40(CmV40DmData),
}

pub trait ExtMetadata {
    fn parse(&mut self, reader: &mut BitVecReader) -> Result<()>;
    fn write(&self, writer: &mut BitVecWriter);
}

pub trait WithExtMetadataBlocks {
    const VERSION: &'static str;
    const ALLOWED_BLOCK_LEVELS: &'static [u8];
    const VARIABLE_LENGTH_BLOCK_LEVELS: &'static [u8];

    fn set_num_ext_blocks(&mut self, num_ext_blocks: u64);
    fn num_ext_blocks(&self) -> u64;

    fn parse_block(&mut self, reader: &mut BitVecReader) -> Result<()>;
    fn blocks_ref(&self) -> &Vec<ExtMetadataBlock>;
    fn blocks_mut(&mut self) -> &mut Vec<ExtMetadataBlock>;

    fn sort_blocks(&mut self) {
        let blocks = self.blocks_mut();
        blocks.sort_by_key(|ext| ext.sort_key());
    }

    fn update_extension_block_info(&mut self) {
        self.set_num_ext_blocks(self.blocks_ref().len() as u64);
        self.sort_blocks();
    }

    fn add_block(&mut self, meta: ExtMetadataBlock) -> Result<()> {
        let level = meta.level();

        ensure!(
            Self::ALLOWED_BLOCK_LEVELS.contains(&level),
            "Metadata block level {} is invalid for {}",
            &level,
            Self::VERSION
        );

        let blocks = self.blocks_mut();
        blocks.push(meta);

        self.update_extension_block_info();

        Ok(())
    }

    fn remove_level(&mut self, level: u8) {
        let blocks = self.blocks_mut();
        blocks.retain(|b| b.level() != level);

        self.update_extension_block_info();
    }

    fn write(&self, writer: &mut BitVecWriter) -> Result<()> {
        let num_ext_blocks = self.num_ext_blocks();

        writer.write_ue(num_ext_blocks);

        // dm_alignment_zero_bit
        while !writer.is_aligned() {
            writer.write(false);
        }

        let ext_metadata_blocks = self.blocks_ref();

        for ext_metadata_block in ext_metadata_blocks {
            let remaining_bits =
                ext_metadata_block.length_bits() - ext_metadata_block.required_bits();

            writer.write_ue(ext_metadata_block.length_bytes());
            writer.write_n(&ext_metadata_block.level().to_be_bytes(), 8);

            ext_metadata_block.write(writer)?;

            // ext_dm_alignment_zero_bit
            (0..remaining_bits).for_each(|_| writer.write(false));
        }

        Ok(())
    }
}

impl DmData {
    pub fn parse<T: WithExtMetadataBlocks + Default>(
        reader: &mut BitVecReader,
    ) -> Result<Option<T>> {
        let mut meta = T::default();
        let num_ext_blocks = reader.get_ue()?;

        meta.set_num_ext_blocks(num_ext_blocks);

        while !reader.is_aligned() {
            ensure!(
                !reader.get()?,
                format!("{}: dm_alignment_zero_bit != 0", T::VERSION)
            );
        }

        for _ in 0..num_ext_blocks {
            meta.parse_block(reader)?;
        }

        Ok(Some(meta))
    }

    pub fn write(&self, writer: &mut BitVecWriter) -> Result<()> {
        match self {
            DmData::V29(m) => m.write(writer),
            DmData::V40(m) => m.write(writer),
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            DmData::V29(m) => m.validate(),
            DmData::V40(m) => m.validate(),
        }
    }
}
