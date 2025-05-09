use std::fmt::Display;

use ez_ffmpeg::{
    stream_info::{find_video_stream_info, StreamInfo},
    FfmpegContext, FfmpegScheduler,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ConverterError {
    #[error("Failed to convert video to audio: {0}")]
    ConversionError(String),
    #[error("No video stream found")]
    NoVideoStreamFound,
    #[error("Failed to get video info: {0}")]
    GetVideoInfoError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub duration: i64,
    pub width: i32,
    pub height: i32,
    pub fps: f64,
}

pub fn get_video_info(video_path: &str) -> Result<VideoInfo, ConverterError> {
    let context = find_video_stream_info(video_path)
        .map_err(|e| ConverterError::GetVideoInfoError(e.to_string()))?;

    if let Some(stream_info) = context {
        if let StreamInfo::Video {
            duration,
            width,
            height,
            fps,
            ..
        } = stream_info
        {
            return Ok(VideoInfo {
                duration,
                width,
                height,
                fps,
            });
        }
    }

    Err(ConverterError::NoVideoStreamFound)
}

pub fn convert_video_to_audio(video_path: &str, audio_path: &str) -> Result<(), ConverterError> {
    let context = FfmpegContext::builder()
        .input(video_path)
        .output(
            ez_ffmpeg::Output::new(audio_path)
                .set_audio_codec("pcm_s16le")
                .set_audio_sample_rate(48000)
                .set_audio_channels(1),
        )
        .build()
        .map_err(|e| ConverterError::ConversionError(e.to_string()))?;

    FfmpegScheduler::new(context)
        .start()
        .map_err(|e| ConverterError::ConversionError(e.to_string()))?
        .wait()
        .map_err(|e| ConverterError::ConversionError(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_convert_video_to_audio() {
        let result = convert_video_to_audio(
            "/Users/jkpark/Parks/99-Record/2025-05-04 17-44-19.mp4",
            "/Users/jkpark/Parks/01-/silent-cut-for-fcp/test22.wav",
        );

        assert_eq!(result.is_ok(), true);
    }
}
