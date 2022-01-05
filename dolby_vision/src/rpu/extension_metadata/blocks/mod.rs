use anyhow::{ensure, Result};
use bitvec_helpers::{bitvec_reader::BitVecReader, bitvec_writer::BitVecWriter};

#[cfg(feature = "serde_feature")]
use serde::{Deserialize, Serialize};

pub mod level1;
pub mod level10;
pub mod level11;
pub mod level2;
pub mod level254;
pub mod level3;
pub mod level4;
pub mod level5;
pub mod level6;
pub mod level8;
pub mod level9;
pub mod reserved;

pub use level1::ExtMetadataBlockLevel1;
pub use level10::ExtMetadataBlockLevel10;
pub use level11::ExtMetadataBlockLevel11;
pub use level2::ExtMetadataBlockLevel2;
pub use level254::ExtMetadataBlockLevel254;
pub use level3::ExtMetadataBlockLevel3;
pub use level4::ExtMetadataBlockLevel4;
pub use level5::ExtMetadataBlockLevel5;
pub use level6::ExtMetadataBlockLevel6;
pub use level8::ExtMetadataBlockLevel8;
pub use level9::ExtMetadataBlockLevel9;
pub use reserved::ReservedExtMetadataBlock;

use super::WithExtMetadataBlocks;

/// cbindgen:ignore
pub const MAX_12_BIT_VALUE: u16 = 4095;
/// cbindgen:ignore
pub const PREDEFINED_COLORSPACE_PRIMARIES: &[[f64; 8]] = &[
    [   0.68,   0.32,  0.265,   0.69,   0.15,   0.06,  0.3127,   0.329], //  0, DCI-P3 D65
    [   0.64,   0.33,   0.30,   0.60,   0.15,   0.06,  0.3127,   0.329], //  1, BT.709
    [  0.708,  0.292,  0.170,  0.797,  0.131,  0.046,  0.3127,   0.329], //  2, BT.2020
    [   0.63,   0.34,   0.31,  0.595,  0.155,   0.07,  0.3127,   0.329], //  3, BT.601 NTSC / SMPTE-C
    [   0.64,   0.33,   0.29,   0.60,   0.15,   0.06,  0.3127,   0.329], //  4, BT.601 PAL / BT.470 BG
    [   0.68,   0.32,  0.265,   0.69,   0.15,   0.06,   0.314,   0.351], //  5, DCI-P3
    [ 0.7347, 0.2653,    0.0,    1.0, 0.0001, -0.077, 0.32168, 0.33767], //  6, ACES
    [   0.73,   0.28,   0.14,  0.855,   0.10,  -0.05,  0.3127,   0.329], //  7, S-Gamut
    [  0.766,  0.275,  0.225,   0.80,  0.089, -0.087,  0.3127,   0.329], //  8, S-Gamut-3.Cine
    ];
/// cbindgen:ignore
pub const PREDEFINED_REALDEVICE_PRIMARIES: &[[f64; 8]] = &[
    [  0.693,  0.304,  0.208,  0.761, 0.1467, 0.0527,  0.3127,   0.329],
    [ 0.6867, 0.3085,  0.231,   0.69, 0.1489, 0.0638,  0.3127,   0.329],
    [ 0.6781, 0.3189, 0.2365, 0.7048,  0.141, 0.0489,  0.3127,   0.329],
    [   0.68,   0.32,  0.265,   0.69,   0.15,   0.06,  0.3127,   0.329],
    [ 0.7042,  0.294, 0.2271,  0.725, 0.1416, 0.0516,  0.3127,   0.329],
    [ 0.6745,  0.310, 0.2212, 0.7109,  0.152, 0.0619,  0.3127,   0.329],
    [ 0.6805, 0.3191, 0.2522, 0.6702, 0.1397, 0.0554,  0.3127,   0.329],
    [ 0.6838, 0.3085, 0.2709, 0.6378, 0.1478, 0.0589,  0.3127,   0.329],
    [ 0.6753, 0.3193, 0.2636, 0.6835, 0.1521, 0.0627,  0.3127,   0.329],
    [ 0.6981, 0.2898, 0.1814, 0.7189, 0.1517, 0.0567,  0.3127,   0.329],
    ];

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_feature", derive(Deserialize, Serialize))]
pub enum ExtMetadataBlock {
    Level1(ExtMetadataBlockLevel1),
    Level2(ExtMetadataBlockLevel2),
    Level3(ExtMetadataBlockLevel3),
    Level4(ExtMetadataBlockLevel4),
    Level5(ExtMetadataBlockLevel5),
    Level6(ExtMetadataBlockLevel6),
    Level8(ExtMetadataBlockLevel8),
    Level9(ExtMetadataBlockLevel9),
    Level10(ExtMetadataBlockLevel10),
    Level11(ExtMetadataBlockLevel11),
    Level254(ExtMetadataBlockLevel254),
    Reserved(ReservedExtMetadataBlock),
}

pub trait ExtMetadataBlockInfo {
    fn level(&self) -> u8;

    fn bits_size(&self) -> u64 {
        self.bytes_size() * 8
    }

    // Block levels with constant length should implement this
    fn bytes_size(&self) -> u64 {
        let index = self
            .possible_required_bits()
            .iter()
            .position(|b| b == &self.required_bits())
            .unwrap();
        return self.possible_bytes_size()[index];
    }

    // Block levels with constant length should implement this
    fn required_bits(&self) -> u64 {
        let count = self.possible_required_bits().len() - 1;
        let last_field_flag = 1 << count >> 1;
        let fields_flag = self.modified_fields_flag();
        let mut bits = self.possible_required_bits().first().unwrap().clone();
        for i in 0..count {
            if fields_flag & (last_field_flag >> i) != 0 {
                bits = self.possible_required_bits()[count - i];
                break;
            };
        };

        return bits;
    }

    // Block levels with variable length should implement this
    // 0b0001 means the first optional field in this block is modified, 
    // 0b0010 for the second field, etc.
    fn modified_fields_flag(&self) -> u64 {
        0
    }

    // Block levels with variable length should implement this
    fn possible_required_bits(&self) -> Vec<u64> {
        vec![self.required_bits()]
    }

    fn possible_bytes_size(&self) -> Vec<u64> {
        return self.possible_required_bits()
        .iter()
        .map(|b| (b + b % 8) >> 3)
        .collect()
    }

    fn possible_bits_size(&self) -> Vec<u64> {
        return self.possible_bytes_size()
            .iter()
            .map(|b| b * 8)
            .collect()
    }

    fn sort_key(&self) -> (u8, u16) {
        (self.level(), 0)
    }
}

impl ExtMetadataBlock {
    pub fn length_bytes(&self) -> u64 {
        match self {
            ExtMetadataBlock::Level1(b) => b.bytes_size(),
            ExtMetadataBlock::Level2(b) => b.bytes_size(),
            ExtMetadataBlock::Level3(b) => b.bytes_size(),
            ExtMetadataBlock::Level4(b) => b.bytes_size(),
            ExtMetadataBlock::Level5(b) => b.bytes_size(),
            ExtMetadataBlock::Level6(b) => b.bytes_size(),
            ExtMetadataBlock::Level8(b) => b.bytes_size(),
            ExtMetadataBlock::Level9(b) => b.bytes_size(),
            ExtMetadataBlock::Level10(b) => b.bytes_size(),
            ExtMetadataBlock::Level11(b) => b.bytes_size(),
            ExtMetadataBlock::Level254(b) => b.bytes_size(),
            ExtMetadataBlock::Reserved(b) => b.bytes_size(),
        }
    }

    pub fn length_bits(&self) -> u64 {
        match self {
            ExtMetadataBlock::Level1(b) => b.bits_size(),
            ExtMetadataBlock::Level2(b) => b.bits_size(),
            ExtMetadataBlock::Level3(b) => b.bits_size(),
            ExtMetadataBlock::Level4(b) => b.bits_size(),
            ExtMetadataBlock::Level5(b) => b.bits_size(),
            ExtMetadataBlock::Level6(b) => b.bits_size(),
            ExtMetadataBlock::Level8(b) => b.bits_size(),
            ExtMetadataBlock::Level9(b) => b.bits_size(),
            ExtMetadataBlock::Level10(b) => b.bits_size(),
            ExtMetadataBlock::Level11(b) => b.bits_size(),
            ExtMetadataBlock::Level254(b) => b.bits_size(),
            ExtMetadataBlock::Reserved(b) => b.bits_size(),
        }
    }

    pub fn required_bits(&self) -> u64 {
        match self {
            ExtMetadataBlock::Level1(b) => b.required_bits(),
            ExtMetadataBlock::Level2(b) => b.required_bits(),
            ExtMetadataBlock::Level3(b) => b.required_bits(),
            ExtMetadataBlock::Level4(b) => b.required_bits(),
            ExtMetadataBlock::Level5(b) => b.required_bits(),
            ExtMetadataBlock::Level6(b) => b.required_bits(),
            ExtMetadataBlock::Level8(b) => b.required_bits(),
            ExtMetadataBlock::Level9(b) => b.required_bits(),
            ExtMetadataBlock::Level10(b) => b.required_bits(),
            ExtMetadataBlock::Level11(b) => b.required_bits(),
            ExtMetadataBlock::Level254(b) => b.required_bits(),
            ExtMetadataBlock::Reserved(b) => b.required_bits(),
        }
    }

    pub fn possible_length_bytes(&self) -> Vec<u64> {
        match self {
            ExtMetadataBlock::Level1(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level2(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level3(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level4(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level5(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level6(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level8(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level9(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level10(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level11(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Level254(b) => b.possible_bytes_size(),
            ExtMetadataBlock::Reserved(b) => b.possible_bytes_size(),
        }
    }

    pub fn possible_length_bits(&self) -> Vec<u64> {
        match self {
            ExtMetadataBlock::Level1(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level2(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level3(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level4(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level5(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level6(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level8(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level9(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level10(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level11(b) => b.possible_bits_size(),
            ExtMetadataBlock::Level254(b) => b.possible_bits_size(),
            ExtMetadataBlock::Reserved(b) => b.possible_bits_size(),
        }
    }

    pub fn possible_required_bits(&self) -> Vec<u64> {
        match self {
            ExtMetadataBlock::Level1(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level2(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level3(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level4(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level5(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level6(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level8(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level9(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level10(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level11(b) => b.possible_required_bits(),
            ExtMetadataBlock::Level254(b) => b.possible_required_bits(),
            ExtMetadataBlock::Reserved(b) => b.possible_required_bits(),
        }
    }

    pub fn level(&self) -> u8 {
        match self {
            ExtMetadataBlock::Level1(b) => b.level(),
            ExtMetadataBlock::Level2(b) => b.level(),
            ExtMetadataBlock::Level3(b) => b.level(),
            ExtMetadataBlock::Level4(b) => b.level(),
            ExtMetadataBlock::Level5(b) => b.level(),
            ExtMetadataBlock::Level6(b) => b.level(),
            ExtMetadataBlock::Level8(b) => b.level(),
            ExtMetadataBlock::Level9(b) => b.level(),
            ExtMetadataBlock::Level10(b) => b.level(),
            ExtMetadataBlock::Level11(b) => b.level(),
            ExtMetadataBlock::Level254(b) => b.level(),
            ExtMetadataBlock::Reserved(b) => b.level(),
        }
    }

    pub fn sort_key(&self) -> (u8, u16) {
        match self {
            ExtMetadataBlock::Level1(b) => b.sort_key(),
            ExtMetadataBlock::Level2(b) => b.sort_key(),
            ExtMetadataBlock::Level3(b) => b.sort_key(),
            ExtMetadataBlock::Level4(b) => b.sort_key(),
            ExtMetadataBlock::Level5(b) => b.sort_key(),
            ExtMetadataBlock::Level6(b) => b.sort_key(),
            ExtMetadataBlock::Level8(b) => b.sort_key(),
            ExtMetadataBlock::Level9(b) => b.sort_key(),
            ExtMetadataBlock::Level10(b) => b.sort_key(),
            ExtMetadataBlock::Level11(b) => b.sort_key(),
            ExtMetadataBlock::Level254(b) => b.sort_key(),
            ExtMetadataBlock::Reserved(b) => b.sort_key(),
        }
    }

    pub fn write(&self, writer: &mut BitVecWriter) -> Result<()> {
        match self {
            ExtMetadataBlock::Level1(b) => b.write(writer),
            ExtMetadataBlock::Level2(b) => b.write(writer),
            ExtMetadataBlock::Level3(b) => b.write(writer),
            ExtMetadataBlock::Level4(b) => b.write(writer),
            ExtMetadataBlock::Level5(b) => b.write(writer),
            ExtMetadataBlock::Level6(b) => b.write(writer),
            ExtMetadataBlock::Level8(b) => b.write(writer),
            ExtMetadataBlock::Level9(b) => b.write(writer),
            ExtMetadataBlock::Level10(b) => b.write(writer),
            ExtMetadataBlock::Level11(b) => b.write(writer),
            ExtMetadataBlock::Level254(b) => b.write(writer),
            ExtMetadataBlock::Reserved(b) => b.write(writer),
        }
    }

    pub fn validate_correct_dm_data<T: WithExtMetadataBlocks>(&self) -> Result<()> {
        let level = self.level();

        ensure!(
            T::ALLOWED_BLOCK_LEVELS.contains(&level),
            "Metadata block level {} is invalid for {}",
            &level,
            T::VERSION
        );

        Ok(())
    }

    pub fn validate_and_read_remaining<T: WithExtMetadataBlocks>(
        &self,
        reader: &mut BitVecReader,
        expected_length: u64,
    ) -> Result<()> {
        let level = self.level();

        ensure!(
            if T::VARIABLE_LENGTH_BLOCK_LEVELS.contains(&level) {
                self.possible_length_bytes().contains(&expected_length)
            } else {
                expected_length == self.length_bytes()
            }
            ,
            format!(
                "{}: Invalid metadata block. Block level {} should have length {}",
                T::VERSION,
                level,
                self.length_bytes()
            )
        );

        self.validate_correct_dm_data::<T>()?;
        
        let mut ext_block_use_bits = 0;
        if T::VARIABLE_LENGTH_BLOCK_LEVELS.contains(&level) {
            if self.possible_length_bytes().contains(&expected_length) {
                let index = self.possible_length_bytes().iter().position(|b| b == &expected_length).unwrap();
                ext_block_use_bits = self.possible_length_bits()[index] - self.possible_required_bits()[index];
            }
        } else {
                ext_block_use_bits = self.length_bits() - self.required_bits();
        }

        for _ in 0..ext_block_use_bits {
            ensure!(
                !reader.get()?,
                format!("{}: ext_dm_alignment_zero_bit != 0", T::VERSION)
            );
        }

        Ok(())
    }
}
