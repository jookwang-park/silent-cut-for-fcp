mod detector;

use detector::analyzer::{Progress, Segment};
use detector::converter::VideoInfo;
use detector::deepfilter::Parameter;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::path::BaseDirectory;
use tauri::{Emitter, Manager};

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
    use_deepfilternet: bool,
    use_normalize: bool,
    target_db: f32,
    peak_normalization: bool,
    threshold_db: f32,
    min_duration_ms: u32,
    left_buffer_sec: f32,
    right_buffer_sec: f32,
    window: tauri::Window,
    handle: tauri::AppHandle,
) -> Result<AnalysisResult, String> {
    let window = Arc::new(window);
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

    if use_deepfilternet {
        let model_path = handle
            .path()
            .resolve("models/DeepFilterNet3_onnx.tar.gz", BaseDirectory::Resource)
            .unwrap();
        println!("model_path: {:?}", model_path);
        let params = Parameter::default();
        let window = window.clone();
        let progress_callback = move |progress: f32| {
            window
                .emit(
                    "analyze-progress",
                    Progress {
                        phase: "Applying DeepFilterNet".to_string(),
                        percentage: progress,
                    },
                )
                .unwrap();
        };
        detector::deepfilter::apply_deepfilternet(
            params,
            &model_path.to_string_lossy(),
            &audio_path,
            &audio_path,
            progress_callback,
        )
        .map_err(|e| e.to_string())?;
    }

    if use_normalize {
        let window = window.clone();
        let analyzer = detector::analyzer::AudioAnalyzer::new();
        let progress_callback = move |progress: Progress| {
            window.emit("analyze-progress", progress).unwrap();
        };

        // 오디오 정규화 실행
        analyzer
            .normalize(
                &audio_path,
                &audio_path,
                detector::analyzer::AudioNormalizerOption {
                    target_db,
                    peak_normalization,
                },
                progress_callback,
            )
            .map_err(|e| e.to_string())?;
    }

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

#[tauri::command]
async fn normalize_audio(
    audio_path: String,
    output_path: String,
    target_db: f32,
    peak_normalization: bool,
    window: tauri::Window,
) -> Result<String, String> {
    let analyzer = detector::analyzer::AudioAnalyzer::new();

    let progress_callback = move |progress: detector::analyzer::Progress| {
        println!("progress: {:?}", progress);
        window.emit("analyze-progress", progress).unwrap();
    };

    // 오디오 정규화 실행
    analyzer
        .normalize(
            &audio_path,
            &output_path,
            detector::analyzer::AudioNormalizerOption {
                target_db,
                peak_normalization,
            },
            progress_callback,
        )
        .map_err(|e| e.to_string())?;

    Ok(output_path)
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
            generate_fcpxml,
            normalize_audio,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
