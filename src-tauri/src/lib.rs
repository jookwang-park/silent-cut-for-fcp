mod detector;

use detector::analyzer::{Progress, Segment};
use detector::converter::VideoInfo;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Emitter;

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisResult {
    segments: Vec<Segment>,
    output_path: String,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_video_info(video_path: String) -> Result<VideoInfo, String> {
    let video_info = detector::converter::get_video_info(&video_path).map_err(|e| e.to_string())?;
    Ok(video_info)
}

#[tauri::command]
async fn analyze_video(
    video_path: String,
    threshold_db: f32,
    min_duration_ms: u32,
    left_buffer_sec: f32,
    right_buffer_sec: f32,
    window: tauri::Window,
) -> Result<AnalysisResult, String> {
    // 임시 오디오 파일 경로 생성
    let video_path_obj = Path::new(&video_path);
    let filename = video_path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("temp");

    let temp_dir = std::env::temp_dir();
    let audio_path = temp_dir
        .join(format!("{}.wav", filename))
        .to_string_lossy()
        .to_string();

    // 비디오에서 오디오 추출
    window
        .emit(
            "analyze-progress",
            Progress {
                phase: "Converting Video to Audio".to_string(),
                percentage: 0.0,
            },
        )
        .unwrap();
    detector::converter::convert_video_to_audio(&video_path, &audio_path)
        .map_err(|e| e.to_string())?;
    window
        .emit(
            "analyze-progress",
            Progress {
                phase: "Converting Video to Audio".to_string(),
                percentage: 100.0,
            },
        )
        .unwrap();

    // 오디오 분석
    let analyzer = detector::analyzer::AudioAnalyzer::new();

    let progress_callback = move |progress: Progress| {
        println!("progress: {:?}", progress);
        window.emit("analyze-progress", progress).unwrap();
    };

    // 소리가 있는 구간 감지
    let segments = analyzer
        .start(
            &audio_path,
            detector::analyzer::AudioAnalyzerOption {
                threshold_db,
                min_duration_ms,
                left_buffer_sec,
                right_buffer_sec,
            },
            progress_callback,
        )
        .map_err(|e| e.to_string())?;
    // 결과 반환
    Ok(AnalysisResult {
        segments,
        output_path: audio_path,
    })
}

#[tauri::command]
async fn generate_fcpxml(
    video_path: String,
    segments: Vec<Segment>,
    fps: String,
    resolution: String,
    output_path: String,
) -> Result<String, String> {
    // FPS 및 해상도 설정
    let fps = match fps.as_str() {
        "29.97" => detector::fcpxml::FPS::FPS29_97,
        "30" => detector::fcpxml::FPS::FPS30,
        "59.94" => detector::fcpxml::FPS::FPS59_94,
        "60" => detector::fcpxml::FPS::FPS60,
        _ => return Err("지원하지 않는 FPS입니다".to_string()),
    };

    let resolution = match resolution.as_str() {
        "SD" => detector::fcpxml::Resolution::SD,
        "HD" => detector::fcpxml::Resolution::HD,
        "FHD" => detector::fcpxml::Resolution::FHD,
        "4K" => detector::fcpxml::Resolution::FourK,
        _ => return Err("지원하지 않는 해상도입니다".to_string()),
    };

    let setting = detector::fcpxml::Setting { fps, resolution };

    // 출력 파일 경로 설정
    let video_path_obj = Path::new(&video_path);
    let filename = video_path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // FCPXML 생성
    detector::fcpxml::generate_fcpxml(setting, &video_path, segments, &output_path)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_video_info,
            analyze_video,
            generate_fcpxml
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
