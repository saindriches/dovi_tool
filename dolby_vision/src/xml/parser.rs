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

    xml_version: String,
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

    fn parse_xml_version(&self, doc: &Document) -> Result<String> {
        if let Some(node) = doc.descendants().find(|e| e.has_tag_name("DolbyLabsMDF")) {
            let version_attr = node.attribute("version");
            let version_node =
                if let Some(version_node) = node.children().find(|e| e.has_tag_name("Version")) {
                    version_node.text()
                } else {
                    None
                };

            let min_version_level254 = if let Some(_) =
                node.descendants().find(|e| e.has_tag_name("Level254"))
            {
                Some("4.0.2")
            } else {
                None
            };

            let min_version_level11 = if let Some(_) =
                node.descendants().find(|e| e.has_tag_name("Level11"))
            {
                Some("5.1.0")
            } else {
                None
            };

            if let Some(v) = version_node {
                let mut checklevel: u8 = 0;
                match v {
                    "5.1.0" => {checklevel += 2}
                    "4.0.2" | "5.0.0" => {checklevel += 1}
                    _ => {
                        bail!("Unknown XML version {} found! Please open an issue.", v);
                    }
                }
                // TODO: Add more checks
                if checklevel >= 2 && min_version_level11.is_none() {
                    bail!("No L11 metadata found in XML version {}!", v);
                }
                if checklevel >= 1 && min_version_level254.is_none() {
                    bail!("No L254 metadata found in XML version {}!", v);
                }
                Ok(v.to_string())
            } else if let Some(v) = version_attr {
                match v {
                    "2.0.5" => {}
                    _ => {
                        bail!("Invalid XML version {} found!", v);
                    }
                }
                Ok(v.to_string())
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

                targets.insert(id.clone(), TargetDisplay { id, peak_nits });
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

    // FIXME: No reference to compare impl
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

        ensure!(trim.len() == 6, "invalid L8 trim: should be 6 values");

        let bias = node
            .children()
            .find(|e| e.has_tag_name("MidContrastBias"))
            .unwrap()
            .text()
            .map_or(0.0, |e| e.parse::<f32>().unwrap());

        let clipping = node
            .children()
            .find(|e| e.has_tag_name("HighlightClipping"))
            .unwrap()
            .text()
            .map_or(0.0, |e| e.parse::<f32>().unwrap());

        let satvec = node
            .children()
            .find(|e| e.has_tag_name("SaturationVectorField"))
            .unwrap()
            .text()
            .unwrap();
        let satvec: Vec<&str> = satvec.split(self.separator).collect();
        let satvec = if satvec.len() != 6 {vec!["0"; 6]} else {satvec};

        let huevec = node
            .children()
            .find(|e| e.has_tag_name("HueVectorField"))
            .unwrap()
            .text()
            .unwrap();
        let huevec: Vec<&str> = huevec.split(self.separator).collect();
        let huevec = if huevec.len() != 6 {vec!["0"; 6]} else {huevec};

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

        let bias = min(
            4095,
            ((bias * 2048.0) + 2048.0).round() as u16,
        );

        let clipping = min(
            4095,
            ((clipping * 2048.0) + 2048.0).round() as u16,
        );

        let satvec: Vec<u8> = satvec
            .iter()
            .map(|v| min(255, ((v.parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8))
            .collect();

        let huevec: Vec<u8> = huevec
            .iter()
            .map(|v| min(255, ((v.parse::<f32>().unwrap() * 128.0) + 128.0).round() as u8))
            .collect();

        let mut block = ExtMetadataBlockLevel8 {
            target_display_index: target_display.id.parse::<u8>()?,
            trim_slope,
            trim_offset,
            trim_power,
            trim_chroma_weight,
            trim_saturation_gain,
            ms_weight,
            ..Default::default()
        };

        let mut bit_flag: u8 = 0b0000;
        if bias != 2048 {
            bit_flag |= 0b0001
        };
        if clipping != 2048 {
            bit_flag |= 0b0010
        };
        if satvec != vec![128; 6] {
            bit_flag |= 0b0100
        };
        if huevec != vec![128; 6] {
            bit_flag |= 0b1000
        };

        if bit_flag & 0b1111 != 0 {
            block.target_mid_contrast = Some(bias)
        };
        if bit_flag & 0b1110 != 0 {
            block.clip_trim = Some(clipping)
        };
        if bit_flag & 0b1100 != 0 {
            block.saturation_vector_field0 = Some(satvec[0]);
            block.saturation_vector_field1 = Some(satvec[1]);
            block.saturation_vector_field2 = Some(satvec[2]);
            block.saturation_vector_field3 = Some(satvec[3]);
            block.saturation_vector_field4 = Some(satvec[4]);
            block.saturation_vector_field5 = Some(satvec[5]);
        };
        if bit_flag & 0b1000 != 0 {
            block.hue_vector_field0 = Some(huevec[0]);
            block.hue_vector_field1 = Some(huevec[1]);
            block.hue_vector_field2 = Some(huevec[2]);
            block.hue_vector_field3 = Some(huevec[3]);
            block.hue_vector_field4 = Some(huevec[4]);
            block.hue_vector_field5 = Some(huevec[5]);
        };

        Ok(block)
    }

    pub fn parse_level9_trim(&self, node: &Node) -> Result<ExtMetadataBlockLevel9> {
        let source_color_model = node
            .children()
            .find(|e| e.has_tag_name("SourceColorModel"))
            .unwrap()
            .text()
            .unwrap();

        let source_primary_index = source_color_model.parse::<u8>()?;

        // TODO
        Ok(ExtMetadataBlockLevel9 {
            source_primary_index,
            ..Default::default()
        })
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
        self.xml_version != "2.0.5"
    }
}
