use anyhow::{bail, ensure, Result};
use roxmltree::{Document, Node};
use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::rpu::extension_metadata::blocks::*;
use crate::rpu::generate::{GenerateConfig, ShotFrameEdit, VideoShot};
use crate::rpu::vdr_dm_data::CmVersion;

#[derive(Default, Debug)]
pub struct CmXmlParser {
    opts: XmlParserOpts,
    xml_version: u16,
    separator: char,

    target_displays: HashMap<String, TargetDisplay>,

    pub config: GenerateConfig,
}

#[derive(Default, Debug)]
pub struct XmlParserOpts {
    pub canvas_width: Option<u16>,
    pub canvas_height: Option<u16>,
}

#[derive(Default, Debug)]
pub struct TargetDisplay {
    id: String,
    peak_nits: u16,
    primaries: String,
}

impl CmXmlParser {
    pub fn parse_file(file_path: &Path, opts: XmlParserOpts) -> Result<CmXmlParser> {
        let mut s = String::new();
        File::open(file_path)?.read_to_string(&mut s)?;

        Self::new(s, opts)
    }

    pub fn new(s: String, opts: XmlParserOpts) -> Result<CmXmlParser> {
        let mut parser = CmXmlParser {
            opts,
            ..Default::default()
        };

        let doc = roxmltree::Document::parse(&s).unwrap();

        parser.xml_version = parser.parse_xml_version(&doc)?;

        parser.separator = if parser.is_cmv4() { ' ' } else { ',' };

        // Override version
        if !parser.is_cmv4() {
            parser.config.cm_version = CmVersion::V29;
        }

        if let Some(output) = doc.descendants().find(|e| e.has_tag_name("Output")) {
            parser.parse_global_level5(&output)?;

            if let Some(video) = output.descendants().find(|e| e.has_tag_name("Video")) {
                let (max_frame_average_light_level, max_content_light_level) =
                    parser.parse_level6(&video);
                let (min_display_mastering_luminance, max_display_mastering_luminance) =
                    parser.parse_mastering_display_metadata(&video);

                parser.config.level6 = ExtMetadataBlockLevel6 {
                    max_display_mastering_luminance,
                    min_display_mastering_luminance,
                    max_content_light_level,
                    max_frame_average_light_level,
                };

                parser.target_displays = parser.parse_target_displays(&video);

                let (dm_mode, dm_version_index) = parser.parse_level254(&video);




                parser.config.shots = parser.parse_shots(&video)?;
                parser.config.shots.sort_by_key(|s| s.start);

                let first_shot = parser.config.shots.first().unwrap();
                let last_shot = parser.config.shots.last().unwrap();

                parser.config.length = (last_shot.start + last_shot.duration) - first_shot.start;
            } else {
                bail!("Could not find Video node");
            }
        } else {
            bail!("Could not find Output node");
        }

        Ok(parser)
    }

    fn parse_xml_version(&self, doc: &Document) -> Result<u16> {
        if let Some(node) = doc.descendants().find(|e| e.has_tag_name("DolbyLabsMDF")) {
            let version_attr = node.attribute("version");
            let version_node =
                if let Some(version_node) = node.children().find(|e| e.has_tag_name("Version")) {
                    version_node.text()
                } else {
                    None
                };

            let mut rev: u16 = 0;

            if let Some(v) = version_attr {
                let ver: Vec<&str> = v.split('.').collect();
                ver
                    .iter()
                    .rev()
                    .enumerate()
                    .for_each(|(i, n)| {
                        rev += n.parse::<u16>().unwrap() << (i * 4);
                    });
                match rev {
                    0x205 => {}
                    0x1 | 0x20 | 0x201 | 0x204 => bail!("Unhandled legacy XML version {} found! Please open an issue.", v),
                    _ => bail!("invalid XML version {} found!", v)
                };
                Ok(rev)
            } else if let Some(v) = version_node {
                let ver: Vec<&str> = v.split('.').collect();
                ver
                    .iter()
                    .rev()
                    .enumerate()
                    .for_each(|(i, n)| {
                        rev += n.parse::<u16>().unwrap() << (i * 4);
                    });
                match rev {
                    0x402 | 0x500 | 0x510 => {}
                    0x510.. => println!("Possibly unhandled new XML version {} found! Please open an issue if you get anything wrong.", v),
                    _ => bail!("invalid XML version {} found!", v)
                };
                Ok(rev)
            } else {
                bail!("No XML version found!");
            }
    } else {
         bail!("Could not find DolbyLabsMDF root node.");
        }
    }

    fn parse_level6(&self, video: &Node) -> (u16, u16) {
        if let Some(node) = video.descendants().find(|e| e.has_tag_name("Level6")) {
            let maxfall = if let Some(fall) = node.children().find(|e| e.has_tag_name("MaxFALL")) {
                fall.text().map_or(0, |e| e.parse::<u16>().unwrap())
            } else {
                0
            };

            let maxcll = if let Some(cll) = node.children().find(|e| e.has_tag_name("MaxCLL")) {
                cll.text().map_or(0, |e| e.parse::<u16>().unwrap())
            } else {
                0
            };

            (maxfall, maxcll)
        } else {
            (0, 0)
        }
    }

    fn parse_mastering_display_metadata(&self, video: &Node) -> (u16, u16) {
        if let Some(node) = video
            .descendants()
            .find(|e| e.has_tag_name("MasteringDisplay"))
        {
            let min = if let Some(min_brightness) = node
                .children()
                .find(|e| e.has_tag_name("MinimumBrightness"))
            {
                min_brightness.text().map_or(0, |e| {
                    let v = e.parse::<f32>().unwrap();
                    (v * 10000.0) as u16
                })
            } else {
                0
            };

            let max = if let Some(max_brightness) =
                node.children().find(|e| e.has_tag_name("PeakBrightness"))
            {
                max_brightness
                    .text()
                    .map_or(0, |e| e.parse::<u16>().unwrap())
            } else {
                0
            };

            (min, max)
        } else {
            (0, 0)
        }
    }

    fn parse_target_displays(&self, video: &Node) -> HashMap<String, TargetDisplay> {
        let mut targets = HashMap::new();

        video
            .descendants()
            .filter(|e| e.has_tag_name("TargetDisplay"))
            .for_each(|e| {
                let id = e
                    .children()
                    .find(|e| e.has_tag_name("ID"))
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_string();

                let peak_nits = e
                    .children()
                    .find(|e| e.has_tag_name("PeakBrightness"))
                    .unwrap()
                    .text()
                    .unwrap()
                    .parse::<u16>()
                    .unwrap();

                if self.xml_version >= 0x500 {
                    let application_type = e
                    .children()
                    .find(|e| e.has_tag_name("ApplicationType"))
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_string();

                    // Only parse HOME targets
                    if application_type == "HOME" {
                        let primary_red = e
                            .descendants()
                            .find(|e| e.has_tag_name("Red"))
                            .unwrap()
                            .text()
                            .unwrap();

                        let primary_green = e
                            .descendants()
                            .find(|e| e.has_tag_name("Green"))
                            .unwrap()
                            .text()
                            .unwrap();

                        let primary_blue = e
                            .descendants()
                            .find(|e| e.has_tag_name("Blue"))
                            .unwrap()
                            .text()
                            .unwrap();

                        let primary_white = e
                            .children()
                            .find(|e| e.has_tag_name("WhitePoint"))
                            .unwrap()
                            .text()
                            .unwrap();
                        
                        let primaries = [primary_red, primary_green, primary_blue,  primary_white]
                            .join(&self.separator.to_string());
                    
                        targets.insert(id.clone(), TargetDisplay { id, peak_nits, primaries });
                    }
                } else {
                    targets.insert(id.clone(), TargetDisplay { id, peak_nits, ..Default::default()});
                }
            });

        targets
    }

    fn parse_level254(&self, video: &Node) -> (u8, u8) {
        if let Some(node) = video.descendants().find(|e| e.has_tag_name("Level254")) {
            let dm_mode = if let Some(dmm) = node.children().find(|e| e.has_tag_name("DMMode")) {
                dmm.text().map_or(0, |e| e.parse::<u8>().unwrap())
            } else {
                0
            };

            let dm_version_index = if let Some(dmv) = node.children().find(|e| e.has_tag_name("DMVersion")) {
                dmv.text().map_or(2, |e| e.parse::<u8>().unwrap())
            } else {
                2
            };

            (dm_mode, dm_version_index)
        } else {
            (0, 2)
        }
    }

    fn parse_shots(&self, video: &Node) -> Result<Vec<VideoShot>> {
        let shots = video
            .descendants()
            .filter(|e| e.has_tag_name("Shot"))
            .map(|n| {
                let mut shot = VideoShot {
                    id: n
                        .children()
                        .find(|e| e.has_tag_name("UniqueID"))
                        .unwrap()
                        .text()
                        .unwrap()
                        .to_string(),
                    ..Default::default()
                };

                if let Some(record) = n.children().find(|e| e.has_tag_name("Record")) {
                    shot.start = record
                        .children()
                        .find(|e| e.has_tag_name("In"))
                        .unwrap()
                        .text()
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    shot.duration = record
                        .children()
                        .find(|e| e.has_tag_name("Duration"))
                        .unwrap()
                        .text()
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                }

                shot.metadata_blocks = self.parse_shot_trims(&n)?;

                let frames = n.children().filter(|e| e.has_tag_name("Frame"));

                for frame in frames {
                    let edit_offset = frame
                        .children()
                        .find(|e| e.has_tag_name("EditOffset"))
                        .unwrap()
                        .text()
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();

                    shot.frame_edits.push(ShotFrameEdit {
                        edit_offset,
                        metadata_blocks: self.parse_shot_trims(&frame)?,
                    });
                }

                Ok(shot)
            })
            .collect();

        shots
    }

    fn parse_shot_trims(&self, node: &Node) -> Result<Vec<ExtMetadataBlock>> {
        let mut metadata_blocks = Vec::new();

        let dynamic_meta_tag = if self.is_cmv4() {
            "DVDynamicData"
        } else {
            "PluginNode"
        };

        if let Some(defaults_node) = node
            .descendants()
            .find(|e| e.has_tag_name(dynamic_meta_tag))
        {
            if self.is_cmv4() {
                let level_nodes = defaults_node
                    .children()
                    .filter(|e| e.has_attribute("level"));

                for level_node in level_nodes {
                    let level = level_node.attribute("level").unwrap();
                    self.parse_trim_levels(&level_node, level, &mut metadata_blocks)?;
                }
            } else {
                let edr_nodes = defaults_node
                    .children()
                    .filter(|e| e.has_tag_name("DolbyEDR") && e.has_attribute("level"));

                for edr in edr_nodes {
                    let level = edr.attribute("level").unwrap();
                    self.parse_trim_levels(&edr, level, &mut metadata_blocks)?;
                }
            };
        }

        Ok(metadata_blocks)
    }

    fn parse_trim_levels(
        &self,
        node: &Node,
        level: &str,
        metadata_blocks: &mut Vec<ExtMetadataBlock>,
    ) -> Result<()> {
        if level == "1" {
            metadata_blocks.push(ExtMetadataBlock::Level1(self.parse_level1_trim(node)?));
        } else if level == "2" {
            metadata_blocks.push(ExtMetadataBlock::Level2(self.parse_level2_trim(node)?));
        } else if level == "3" {
            metadata_blocks.push(ExtMetadataBlock::Level3(self.parse_level3_trim(node)?));
        } else if level == "5" {
            metadata_blocks.push(ExtMetadataBlock::Level5(self.parse_level5_trim(node)?));
        } else if level == "8" {
            metadata_blocks.push(ExtMetadataBlock::Level8(self.parse_level8_trim(node)?));
        } else if level == "9" {
            metadata_blocks.push(ExtMetadataBlock::Level9(self.parse_level9_trim(node)?));
        }

        Ok(())
    }

    pub fn parse_global_level5(&mut self, output: &Node) -> Result<()> {
        let canvas_ar = if let Some(canvas_ar) = output
            .children()
            .find(|e| e.has_tag_name("CanvasAspectRatio"))
        {
            canvas_ar.text().and_then(|v| v.parse::<f32>().ok())
        } else {
            None
        };

        let image_ar = if let Some(image_ar) = output
            .children()
            .find(|e| e.has_tag_name("ImageAspectRatio"))
        {
            image_ar.text().and_then(|v| v.parse::<f32>().ok())
        } else {
            None
        };

        if let (Some(c_ar), Some(i_ar)) = (canvas_ar, image_ar) {
            self.config.level5 = self
                .calculate_level5_metadata(c_ar, i_ar)
                .ok()
                .unwrap_or_default();
        }

        Ok(())
    }

    pub fn parse_level1_trim(&self, node: &Node) -> Result<ExtMetadataBlockLevel1> {
        let measurements = node
            .children()
            .find(|e| e.has_tag_name("ImageCharacter"))
            .unwrap()
            .text()
            .unwrap();
        let measurements: Vec<&str> = measurements.split(self.separator).collect();

        ensure!(
            measurements.len() == 3,
            "invalid L1 trim: should be 3 values"
        );

        let min_pq = (measurements[0].parse::<f32>().unwrap() * 4095.0).round() as u16;
        let avg_pq = (measurements[1].parse::<f32>().unwrap() * 4095.0).round() as u16;
        let max_pq = (measurements[2].parse::<f32>().unwrap() * 4095.0).round() as u16;

        Ok(ExtMetadataBlockLevel1::from_stats(min_pq, max_pq, avg_pq))
    }

    pub fn parse_level2_trim(&self, node: &Node) -> Result<ExtMetadataBlockLevel2> {
        let target_id = node
            .children()
            .find(|e| e.has_tag_name("TID"))
            .unwrap()
            .text()
            .unwrap()
            .to_string();

        let trim = node
            .children()
            .find(|e| e.has_tag_name("Trim"))
            .unwrap()
            .text()
            .unwrap();
        let trim: Vec<&str> = trim.split(self.separator).collect();

        let target_display = self
            .target_displays
            .get(&target_id)
            .expect("No target display found for L2 trim");

        ensure!(trim.len() == 9, "invalid L2 trim: should be 9 values");

        let trim_lift = trim[3].parse::<f32>().unwrap();
        let trim_gain = trim[4].parse::<f32>().unwrap();
        let trim_gamma = trim[5].parse::<f32>().unwrap().clamp(-1.0, 1.0);

        let trim_slope = min(
            4095,
            ((((trim_gain + 2.0) * (1.0 - trim_lift / 2.0) - 2.0) * 2048.0) + 2048.0).round()
                as u16,
        );
        let trim_offset = min(
            4095,
            ((((trim_gain + 2.0) * (trim_lift / 2.0)) * 2048.0) + 2048.0).round() as u16,
        );
        let trim_power = min(
            4095,
            (((2.0 / (1.0 + trim_gamma / 2.0) - 2.0) * 2048.0) + 2048.0).round() as u16,
        );
        let trim_chroma_weight = min(
            4095,
            ((trim[6].parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as u16,
        );
        let trim_saturation_gain = min(
            4095,
            ((trim[7].parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as u16,
        );
        let ms_weight = min(
            4095,
            ((trim[8].parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as i16,
        );

        Ok(ExtMetadataBlockLevel2 {
            trim_slope,
            trim_offset,
            trim_power,
            trim_chroma_weight,
            trim_saturation_gain,
            ms_weight,
            ..ExtMetadataBlockLevel2::from_nits(target_display.peak_nits)
        })
    }

    pub fn parse_level3_trim(&self, node: &Node) -> Result<ExtMetadataBlockLevel3> {
        let measurements = node
            .children()
            .find(|e| e.has_tag_name("L1Offset"))
            .unwrap()
            .text()
            .unwrap();
        let measurements: Vec<&str> = measurements.split(self.separator).collect();

        ensure!(
            measurements.len() == 3,
            "invalid L3 trim: should be 3 values"
        );

        Ok(ExtMetadataBlockLevel3 {
            min_pq_offset: ((measurements[0].parse::<f32>().unwrap() * 2048.0) + 2048.0).round()
                as u16,
            max_pq_offset: ((measurements[1].parse::<f32>().unwrap() * 2048.0) + 2048.0).round()
                as u16,
            avg_pq_offset: ((measurements[2].parse::<f32>().unwrap() * 2048.0) + 2048.0).round()
                as u16,
        })
    }

    pub fn parse_level5_trim(&self, node: &Node) -> Result<ExtMetadataBlockLevel5> {
        let ratios = node
            .children()
            .find(|e| e.has_tag_name("AspectRatios"))
            .unwrap()
            .text()
            .unwrap();
        let ratios: Vec<&str> = ratios.split(self.separator).collect();

        ensure!(ratios.len() == 2, "invalid L5 trim: should be 2 values");

        let canvas_ar = ratios[0].parse::<f32>().unwrap();
        let image_ar = ratios[1].parse::<f32>().unwrap();

        Ok(self
            .calculate_level5_metadata(canvas_ar, image_ar)
            .ok()
            .unwrap_or_default())
    }

    pub fn parse_level8_trim(&self, node: &Node) -> Result<ExtMetadataBlockLevel8> {
        let target_id = node
            .children()
            .find(|e| e.has_tag_name("TID"))
            .unwrap()
            .text()
            .unwrap()
            .to_string();

        let trim = node
            .children()
            .find(|e| e.has_tag_name("L8Trim"))
            .unwrap()
            .text()
            .unwrap();
        let trim: Vec<&str> = trim.split(self.separator).collect();

        let target_display = self
            .target_displays
            .get(&target_id)
            .expect("No target display found for L8 trim");

        ensure!(trim.len() == 6, "Invalid L8 trim: should be 6 values");

        let bias = node
            .children()
            .find(|e| e.has_tag_name("MidContrastBias"))
            .unwrap()
            .text()
            .unwrap();

        let clipping = node
            .children()
            .find(|e| e.has_tag_name("HighlightClipping"))
            .unwrap()
            .text()
            .unwrap();

        let satvec = node
            .children()
            .find(|e| e.has_tag_name("SaturationVectorField"))
            .unwrap()
            .text()
            .unwrap();
            
        let satvec: Vec<&str> = satvec.split(self.separator).collect();
        ensure!(satvec.len() == 6, "Invalid L8 SatVectorField: should be 6 values");

        let huevec = node
            .children()
            .find(|e| e.has_tag_name("HueVectorField"))
            .unwrap()
            .text()
            .unwrap();

        let huevec: Vec<&str> = huevec.split(self.separator).collect();
        ensure!(huevec.len() == 6, "Invalid L8 HueVectorField: should be 6 values");

        let trim_lift = trim[0].parse::<f32>().unwrap();
        let trim_gain = trim[1].parse::<f32>().unwrap();
        let trim_gamma = trim[2].parse::<f32>().unwrap().clamp(-1.0, 1.0);

        let trim_slope = min(
            4095,
            ((((trim_gain + 2.0) * (1.0 - trim_lift / 2.0) - 2.0) * 2048.0) + 2048.0).round()
                as u16,
        );
        let trim_offset = min(
            4095,
            ((((trim_gain + 2.0) * (trim_lift / 2.0)) * 2048.0) + 2048.0).round() as u16,
        );
        let trim_power = min(
            4095,
            (((2.0 / (1.0 + trim_gamma / 2.0) - 2.0) * 2048.0) + 2048.0).round() as u16,
        );
        let trim_chroma_weight = min(
            4095,
            ((trim[3].parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as u16,
        );
        let trim_saturation_gain = min(
            4095,
            ((trim[4].parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as u16,
        );
        let ms_weight = min(
            4095,
            ((trim[5].parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as u16,
        );

        let target_mid_contrast = min(
            4095,
            ((bias.parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as u16,
        );

        let clip_trim = min(
            4095,
            ((clipping.parse::<f32>().unwrap() * 2048.0) + 2048.0).round() as u16,
        );

        let saturation_vector_field0 = min(
            255, ((satvec[0].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let saturation_vector_field1 = min(
            255, ((satvec[1].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let saturation_vector_field2 = min(
            255, ((satvec[2].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let saturation_vector_field3 = min(
            255, ((satvec[3].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let saturation_vector_field4 = min(
            255, ((satvec[4].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let saturation_vector_field5 = min(
            255, ((satvec[5].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let hue_vector_field0 = min(
            255, ((huevec[0].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let hue_vector_field1 = min(
            255, ((huevec[1].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let hue_vector_field2 = min(
            255, ((huevec[2].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let hue_vector_field3 = min(
            255, ((huevec[3].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let hue_vector_field4 = min(
            255, ((huevec[4].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        let hue_vector_field5 = min(
            255, ((huevec[5].parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8,
        );

        Ok (ExtMetadataBlockLevel8 {
            target_display_index: target_display.id.parse::<u8>()?,
            trim_slope,
            trim_offset,
            trim_power,
            trim_chroma_weight,
            trim_saturation_gain,
            ms_weight,
            target_mid_contrast,
            clip_trim,
            saturation_vector_field0,
            saturation_vector_field1,
            saturation_vector_field2,
            saturation_vector_field3,
            saturation_vector_field4,
            saturation_vector_field5,
            hue_vector_field0,
            hue_vector_field1,
            hue_vector_field2,
            hue_vector_field3,
            hue_vector_field4,
            hue_vector_field5,
        })
    }

    fn parse_primary_index(&self, primaries: &Vec<&str>, is_source: bool) -> Result<u8> {
        fn compare_primaries(a: &Vec<&str>, b: &[f64; 8], compare_flag: &mut u8) -> bool {
            *compare_flag = 0;
            for i in 0..8 {
                if (a[i].parse::<f64>().unwrap() - b[i]).abs() < f64::EPSILON {
                    *compare_flag |= 1 << i;
                } else {
                    break;
                }
            };
            if *compare_flag == 0b11111111 {
                return true;
            } else {
                return false;
            }
        }

        let mut result: u8 = 0;
        let mut compare_flag: u8 = 0;
        for p in PREDEFINED_COLORSPACE_PRIMARIES {
            if compare_primaries(primaries, p, &mut compare_flag) {
                break;
            } else {
                result += 1;
            };
        };
        if compare_flag != 0b11111111 && is_source {
            for p in PREDEFINED_REALDEVICE_PRIMARIES {
                if compare_primaries(primaries, p, &mut compare_flag) {
                    break;
                } else {
                    result += 1;
                };
            };
        };
        if compare_flag != 0b11111111 {
            result = 255;
        }
        Ok(result)
    }

    pub fn parse_level9_trim(&self, node: &Node) -> Result<ExtMetadataBlockLevel9> {
        let source_color_primary = node
            .children()
            .find(|e| e.has_tag_name("SourceColorPrimary"))
            .unwrap()
            .text()
            .unwrap();
            
            let primaries: Vec<&str> = source_color_primary.split(self.separator).collect();
            ensure!(primaries.len() == 8, "Invalid L9 SourceColorPrimary: should be 8 values");
            let index = self.parse_primary_index(&primaries, true)?;

            let mut block = ExtMetadataBlockLevel9 {
                source_primary_index: index,
              ..Default::default()
            };

            if index == 255 {
                let p: Vec<u16> = primaries
                    .iter()
                    .map(|v||i| -> u16 {
                            match i {
                                // This value will not be 32768
                                32767.. => min(32767, i - 32767),
                                _ => i + 32769,
                            }
                        } ((v.parse::<f64>().unwrap() * 32767.0 + 32767.0).round() as u16)
                    )
                    .collect();
                
                block.source_primary_red_x =p[0];
                block.source_primary_red_y = p[1];
                block.source_primary_green_x = p[2];
                block.source_primary_green_y = p[3];
                block.source_primary_blue_x = p[4];
                block.source_primary_blue_y = p[5];
                block.source_primary_white_x = p[6];
                block.source_primary_white_y = p[7];
            }

        Ok(block)
    }

    fn calculate_level5_metadata(
        &self,
        canvas_ar: f32,
        image_ar: f32,
    ) -> Result<ExtMetadataBlockLevel5> {
        ensure!(
            self.opts.canvas_width.is_some(),
            "Missing canvas width to calculate L5"
        );
        ensure!(
            self.opts.canvas_height.is_some(),
            "Missing canvas height to calculate L5"
        );

        let cw = self.opts.canvas_width.unwrap() as f32;
        let ch = self.opts.canvas_height.unwrap() as f32;

        let mut calculated_level5 = ExtMetadataBlockLevel5::default();

        if (canvas_ar - image_ar).abs() < f32::EPSILON {
            // No AR difference, zero offsets
        } else if image_ar > canvas_ar {
            let image_h = (ch * (canvas_ar / image_ar)).round();
            let diff = ch - image_h;
            let offset_top = (diff / 2.0).trunc();
            let offset_bottom = diff - offset_top;

            calculated_level5.active_area_top_offset = offset_top as u16;
            calculated_level5.active_area_bottom_offset = offset_bottom as u16;
        } else {
            let image_w = (cw * (image_ar / canvas_ar)).round();
            let diff = cw - image_w;
            let offset_left = (diff / 2.0).trunc();
            let offset_right = diff - offset_left;

            calculated_level5.active_area_left_offset = offset_left as u16;
            calculated_level5.active_area_right_offset = offset_right as u16;
        }

        Ok(calculated_level5)
    }

    pub fn is_cmv4(&self) -> bool {
        self.xml_version >= 0x402
    }
}
