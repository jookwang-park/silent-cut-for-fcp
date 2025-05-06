use std::{fs::File, io::Write, path::Display};

use serde::{Deserialize, Serialize};
use symphonia::core::units::Duration;

use super::analyzer::Segment;

#[derive(thiserror::Error, Debug)]
pub enum FcpXmlError {
    #[error("Failed to generate FCP XML: {0}")]
    GenerateError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FPS {
    FPS29_97,
    FPS30,
    FPS59_94,
    FPS60,
}

impl std::fmt::Display for FPS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FPS::FPS29_97 => write!(f, "2997"),
            FPS::FPS30 => write!(f, "30"),
            FPS::FPS59_94 => write!(f, "5994"),
            FPS::FPS60 => write!(f, "60"),
        }
    }
}

impl FPS {
    pub fn to_frame_duration(&self) -> String {
        match self {
            FPS::FPS29_97 => "1001/30000s".to_string(),
            FPS::FPS30 => "1000/30000s".to_string(),
            FPS::FPS59_94 => "1001/60000s".to_string(),
            FPS::FPS60 => "1000/60000s".to_string(),
        }
    }

    pub fn num(&self) -> i32 {
        match self {
            FPS::FPS29_97 => 1001,
            FPS::FPS30 => 1000,
            FPS::FPS59_94 => 1001,
            FPS::FPS60 => 1000,
        }
    }

    pub fn denom(&self) -> i32 {
        match self {
            FPS::FPS29_97 => 30000,
            FPS::FPS30 => 30000,
            FPS::FPS59_94 => 60000,
            FPS::FPS60 => 60000,
        }
    }

    pub fn get_frame_count(&self, time_seconds: f32) -> i64 {
        // 정수 프레임 수 계산 (반올림)
        let frames = (time_seconds * (self.denom() as f32 / self.num() as f32)).round() as i64;
        frames
    }

    pub fn get_start_timecode(&self, start_frame: i64) -> String {
        format!("{}/{}s", start_frame * self.num() as i64, self.denom())
    }

    pub fn get_offset_timecode(&self, current_frame_offset: i64) -> String {
        format!(
            "{}/{}s",
            current_frame_offset * self.num() as i64,
            self.denom()
        )
    }

    pub fn get_duration_timecode(&self, duration_frame: i64) -> String {
        format!("{}/{}s", duration_frame * self.num() as i64, self.denom())
    }

    pub fn get_duration_frame(&self, start_frame: i64, end_frame: i64) -> i64 {
        end_frame - start_frame
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    SD,
    HD,
    FHD,
    FourK,
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resolution::SD => write!(f, "FFVideoFormat480p"),
            Resolution::HD => write!(f, "FFVideoFormat720p"),
            Resolution::FHD => write!(f, "FFVideoFormat1080p"),
            Resolution::FourK => write!(f, "FFVideoFormat2160p"),
        }
    }
}

impl Resolution {
    pub fn get_width(&self) -> i32 {
        match self {
            Resolution::SD => 640,
            Resolution::HD => 1280,
            Resolution::FHD => 1920,
            Resolution::FourK => 3840,
        }
    }

    pub fn get_height(&self) -> i32 {
        match self {
            Resolution::SD => 480,
            Resolution::HD => 720,
            Resolution::FHD => 1080,
            Resolution::FourK => 2160,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub fps: FPS,
    pub resolution: Resolution,
}

pub fn generate_fcpxml(
    setting: Setting,
    input_video_path: &str,
    silent_segments: Vec<Segment>,
    output_path: &str,
) -> Result<String, FcpXmlError> {
    let mut xml = xml_builder::XMLBuilder::new()
        .version(xml_builder::XMLVersion::XML1_0)
        .encoding("UTF-8".to_string())
        .build();

    let mut root = xml_builder::XMLElement::new("fcpxml");
    root.add_attribute("version", "1.13");

    // resources
    let mut resources = xml_builder::XMLElement::new("resources");

    let mut format = xml_builder::XMLElement::new("format");
    let resource_name = format!("{}{}", setting.resolution, setting.fps);
    format.add_attribute("id", "r1");
    format.add_attribute("name", resource_name.as_str());
    format.add_attribute("frameDuration", setting.fps.to_frame_duration().as_str());
    format.add_attribute("width", setting.resolution.get_width().to_string().as_str());
    format.add_attribute(
        "height",
        setting.resolution.get_height().to_string().as_str(),
    );

    let mut asset = xml_builder::XMLElement::new("asset");
    asset.add_attribute("id", "r2");
    asset.add_attribute("name", "Video");
    asset.add_attribute("hasVideo", "1");
    asset.add_attribute("hasAudio", "1");

    let mut media_rep = xml_builder::XMLElement::new("media-rep");
    media_rep.add_attribute("kind", "original-media");
    media_rep.add_attribute("src", input_video_path);

    asset
        .add_child(media_rep)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    resources
        .add_child(format)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;
    resources
        .add_child(asset)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    // event
    let mut event = xml_builder::XMLElement::new("event");
    event.add_attribute("name", "Clipping");

    // project
    let mut project = xml_builder::XMLElement::new("project");
    project.add_attribute("name", "Clipping");

    // sequence
    let mut sequence = xml_builder::XMLElement::new("sequence");
    sequence.add_attribute("format", "r1");
    sequence.add_attribute("tcStart", "0/1s");

    // spine
    let mut spine = xml_builder::XMLElement::new("spine");

    let mut current_frame_offset: i64 = 0;
    for segment in silent_segments {
        let start = segment.start;
        let end = segment.end;
        let start_frame = setting.fps.get_frame_count(start);
        let end_frame = setting.fps.get_frame_count(end);
        let duration_frame = setting.fps.get_duration_frame(start_frame, end_frame);

        let mut asset_clip = xml_builder::XMLElement::new("asset-clip");
        asset_clip.add_attribute("ref", "r2");
        asset_clip.add_attribute(
            "offset",
            setting
                .fps
                .get_offset_timecode(current_frame_offset)
                .as_str(),
        );
        asset_clip.add_attribute("name", "Video");
        asset_clip.add_attribute(
            "duration",
            setting.fps.get_duration_timecode(duration_frame).as_str(),
        );
        asset_clip.add_attribute(
            "start",
            setting.fps.get_start_timecode(start_frame).as_str(),
        );

        current_frame_offset += duration_frame;

        spine
            .add_child(asset_clip)
            .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;
    }

    sequence
        .add_child(spine)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    project
        .add_child(sequence)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    event
        .add_child(project)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    root.add_child(resources)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;
    root.add_child(event)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    xml.set_root_element(root);

    let mut writer = Vec::new();
    xml.generate(&mut writer)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    let mut file =
        File::create(output_path).map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;
    file.write_all(&writer)
        .map_err(|e| FcpXmlError::GenerateError(e.to_string()))?;

    Ok(output_path.to_string())
}
