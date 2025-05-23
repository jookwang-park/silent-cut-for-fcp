use std::{fs::File, path::Path};

use hound;
use serde::{Deserialize, Serialize};
use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

#[derive(Debug, thiserror::Error)]
pub enum AudioAnalyzerError {
    #[error("Failed to read audio file: {0}")]
    ReadError(String),
    #[error("Failed to analyze audio file: {0}")]
    AnalysisError(String),
    #[error("Failed to decode audio: {0}")]
    DecodeError(String),
    #[error("Failed to write audio file: {0}")]
    WriteError(String),
}

struct ProcessedAudio {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

pub struct AudioAnalyzerOption {
    pub threshold_db: f32,
    pub min_duration_ms: u32,
    pub left_buffer_sec: f32,
    pub right_buffer_sec: f32,
}

impl Default for AudioAnalyzerOption {
    fn default() -> Self {
        Self {
            threshold_db: 20.0,
            min_duration_ms: 50,
            left_buffer_sec: 0.01,
            right_buffer_sec: 0.15,
        }
    }
}

pub struct AudioNormalizerOption {
    pub target_db: f32,
    pub peak_normalization: bool,
}

impl Default for AudioNormalizerOption {
    fn default() -> Self {
        Self {
            target_db: -3.0,
            peak_normalization: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub phase: String,
    pub percentage: f32,
}

pub struct AudioAnalyzer {}

impl AudioAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    fn process_audio_samples<F>(
        &self,
        audio_path: &str,
        progress_callback: &mut F,
    ) -> Result<ProcessedAudio, AudioAnalyzerError>
    where
        F: FnMut(Progress) -> () + Send + Sync + 'static,
    {
        progress_callback(Progress {
            phase: "Processing Audio".to_string(),
            percentage: 0.0,
        });

        let file =
            File::open(&audio_path).map_err(|e| AudioAnalyzerError::ReadError(e.to_string()))?;
        let media_source = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = Path::new(&audio_path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts = MetadataOptions::default();
        let probed = symphonia::default::get_probe()
            .format(&hint, media_source, &format_opts, &metadata_opts)
            .map_err(|e| AudioAnalyzerError::AnalysisError(e.to_string()))?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or(AudioAnalyzerError::AnalysisError(
                "No default track".to_string(),
            ))?;
        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &Default::default())
            .map_err(|e| AudioAnalyzerError::DecodeError(e.to_string()))?;
        let mut sample_rate = 0;
        let mut samples: Vec<f32> = Vec::new();

        // 프로그레스 출력을 위한 변수
        let total_frames = decoder.codec_params().n_frames;
        let mut processed_frames = 0;

        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }

            let decoded = decoder
                .decode(&packet)
                .map_err(|e| AudioAnalyzerError::DecodeError(e.to_string()))?;

            if sample_rate == 0 {
                sample_rate = decoded.spec().rate;
            }

            match decoded {
                AudioBufferRef::S16(buffer) => match buffer.spec().channels.count() {
                    1 => {
                        samples.extend(buffer.chan(0).iter().copied().map(|s| s as f32));
                        processed_frames += buffer.chan(0).len();
                    }
                    2 => {
                        for i in 0..buffer.frames() {
                            let left = buffer.chan(0)[i];
                            let right = buffer.chan(1)[i];
                            let average = (left as f32 + right as f32) / 2.0;
                            samples.push(average);

                            processed_frames += 1;
                        }
                    }
                    n => {
                        for i in 0..buffer.frames() {
                            let mut sum = 0.0;
                            for ch in 0..n {
                                sum += buffer.chan(ch)[i] as f32;
                            }
                            samples.push(sum / n as f32);

                            processed_frames += 1;
                        }
                    }
                },
                _ => {}
            }

            if let Some(total_frames) = total_frames {
                progress_callback(Progress {
                    phase: "Processing Audio".to_string(),
                    percentage: processed_frames as f32 / total_frames as f32 * 100.0,
                });
            }
        }

        Ok(ProcessedAudio {
            sample_rate,
            samples,
        })
    }

    // 오디오 정규화 함수
    pub fn normalize<F>(
        &self,
        input_path: &str,
        output_path: &str,
        AudioNormalizerOption {
            target_db,
            peak_normalization,
        }: AudioNormalizerOption,
        mut progress_callback: F,
    ) -> Result<(), AudioAnalyzerError>
    where
        F: FnMut(Progress) -> () + Send + Sync + 'static,
    {
        // 오디오 파일 로드
        progress_callback(Progress {
            phase: "Normalizing Audio".to_string(),
            percentage: 0.0,
        });

        let ProcessedAudio {
            sample_rate,
            mut samples,
        } = self.process_audio_samples(input_path, &mut progress_callback)?;

        // 1. 최대 진폭 또는 RMS 값 계산
        progress_callback(Progress {
            phase: "Normalizing Audio".to_string(),
            percentage: 33.0,
        });

        let target_amplitude = 10.0_f32.powf(target_db / 20.0);
        let mut normalization_factor = 1.0;

        if peak_normalization {
            // 최대 진폭을 기준으로 정규화
            let max_amplitude = samples.iter().map(|&s| s.abs()).fold(0.0, f32::max);

            if max_amplitude > 0.0 {
                normalization_factor = target_amplitude / max_amplitude;
            }
        } else {
            // RMS 값을 기준으로 정규화
            let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
            let rms = (sum_squares / samples.len() as f32).sqrt();

            if rms > 0.0 {
                normalization_factor = target_amplitude / rms;
            }
        }

        // 2. 모든 샘플에 정규화 계수 적용
        progress_callback(Progress {
            phase: "Normalizing Audio".to_string(),
            percentage: 66.0,
        });

        for sample in &mut samples {
            *sample *= normalization_factor;

            // 클리핑 방지 (최대값을 넘지 않도록)
            if *sample > 1.0 {
                *sample = 1.0;
            } else if *sample < -1.0 {
                *sample = -1.0;
            }
        }

        // 3. 정규화된 오디오 파일 저장
        progress_callback(Progress {
            phase: "Normalizing Audio".to_string(),
            percentage: 90.0,
        });

        // hound 라이브러리를 사용하여 WAV 파일 저장
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(output_path, spec)
            .map_err(|e| AudioAnalyzerError::WriteError(e.to_string()))?;

        // 샘플 데이터 작성
        for sample in samples {
            // f32 샘플을 i16으로 변환 (PCM 16-bit)
            let pcm_sample = (sample * 32767.0) as i16;
            writer
                .write_sample(pcm_sample)
                .map_err(|e| AudioAnalyzerError::WriteError(e.to_string()))?;
        }

        writer
            .finalize()
            .map_err(|e| AudioAnalyzerError::WriteError(e.to_string()))?;

        progress_callback(Progress {
            phase: "Normalizing Audio".to_string(),
            percentage: 100.0,
        });

        Ok(())
    }

    fn find_non_silent_segments<F>(
        &self,
        ProcessedAudio {
            sample_rate,
            samples,
        }: ProcessedAudio,
        threshold_db: f32,
        min_duration_ms: u32,
        left_buffer_sec: f32,
        right_buffer_sec: f32,
        progress_callback: &mut F,
    ) -> Vec<Segment>
    where
        F: FnMut(Progress) -> () + Send + Sync + 'static,
    {
        let threshold_amplitude = 10.0_f32.powf(threshold_db / 20.0);

        let left_buffer_samples = (sample_rate as f32 * left_buffer_sec) as usize;
        let right_buffer_samples = (sample_rate as f32 * right_buffer_sec) as usize;

        let window_size = (sample_rate / 100) as usize;
        let min_samples = (sample_rate as u32 * min_duration_ms / 1000) as usize;

        let mut rms_values = Vec::with_capacity(samples.len() / window_size + 1);

        progress_callback(Progress {
            phase: "Analyzing Audio".to_string(),
            percentage: 0.0,
        });

        let chunk_count = samples.chunks(window_size).len();
        for (i, chunk) in samples.chunks(window_size).enumerate() {
            if !chunk.is_empty() {
                let sum_squares: f32 = chunk.iter().map(|&s| s * s).sum();
                let rms = (sum_squares / chunk.len() as f32).sqrt();
                rms_values.push(rms);
            }

            // RMS 계산 진행률 업데이트 (0 ~ 40%)
            if i % (chunk_count / 20).max(1) == 0 {
                let progress = (i as f32 / chunk_count as f32) * 40.0;
                progress_callback(Progress {
                    phase: "Analyzing Audio".to_string(),
                    percentage: progress,
                });
            }
        }

        // RMS 계산 완료 (40%)
        progress_callback(Progress {
            phase: "Analyzing Audio".to_string(),
            percentage: 40.0,
        });

        let mut segments = Vec::new();
        let mut is_non_silent = false;
        let mut silent_start_idx = 0;

        if !rms_values.is_empty() && rms_values[0] >= threshold_amplitude {
            is_non_silent = true;
            silent_start_idx = 0;
        }

        // 세그먼트 검색 진행률 업데이트 (40 ~ 80%)
        let rms_len = rms_values.len();
        for (i, &rms) in rms_values.iter().enumerate() {
            if rms >= threshold_amplitude && !is_non_silent {
                is_non_silent = true;
                silent_start_idx = i;
            } else if rms < threshold_amplitude && is_non_silent {
                is_non_silent = false;
                let duration_in_windows = i - silent_start_idx;
                let duration_in_samples = duration_in_windows * window_size;

                if duration_in_samples >= min_samples {
                    let buffered_start_idx = if silent_start_idx > 0 {
                        let buffer_windows = left_buffer_samples / window_size;
                        silent_start_idx.saturating_sub(buffer_windows)
                    } else {
                        0
                    };

                    let buffered_end_idx =
                        (i + right_buffer_samples / window_size).min(rms_values.len());

                    let start_time = (buffered_start_idx * window_size) as f32 / sample_rate as f32;
                    let end_time = (buffered_end_idx * window_size).min(samples.len()) as f32
                        / sample_rate as f32;

                    segments.push(Segment {
                        start: start_time,
                        end: end_time,
                    });
                }
            }

            // 세그먼트 검색 진행률 업데이트
            if i % (rms_len / 20).max(1) == 0 {
                let progress = 40.0 + (i as f32 / rms_len as f32) * 40.0;
                progress_callback(Progress {
                    phase: "Analyzing Audio".to_string(),
                    percentage: progress,
                });
            }
        }

        if is_non_silent {
            let duration_in_windows = rms_values.len() - silent_start_idx;
            let duration_in_samples = duration_in_windows * window_size;

            if duration_in_samples >= min_samples {
                let buffered_start_idx = if silent_start_idx > 0 {
                    let buffer_windows = left_buffer_samples / window_size;
                    silent_start_idx.saturating_sub(buffer_windows)
                } else {
                    0
                };

                let start_time = (buffered_start_idx * window_size) as f32 / sample_rate as f32;
                let end_time = samples.len() as f32 / sample_rate as f32;

                segments.push(Segment {
                    start: start_time,
                    end: end_time,
                });
            }
        }

        // 세그먼트 검색 완료 (80%)
        progress_callback(Progress {
            phase: "Analyzing Audio".to_string(),
            percentage: 80.0,
        });

        if segments.is_empty() {
            progress_callback(Progress {
                phase: "Analyzing Audio".to_string(),
                percentage: 100.0,
            });
            return segments;
        }

        segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

        let mut merged_segments = Vec::with_capacity(segments.len());
        let mut current_segment = segments[0];

        for &segment in segments.iter().skip(1) {
            if segment.start <= current_segment.end + (0.1 * left_buffer_sec) {
                current_segment.end = segment.end.max(current_segment.end);
            } else {
                merged_segments.push(current_segment);
                current_segment = segment;
            }
        }

        merged_segments.push(current_segment);

        // 완료 (100%)
        progress_callback(Progress {
            phase: "Analyzing Audio".to_string(),
            percentage: 100.0,
        });

        merged_segments
    }

    pub fn start<F>(
        &self,
        audio_path: &str,
        AudioAnalyzerOption {
            threshold_db,
            min_duration_ms,
            left_buffer_sec,
            right_buffer_sec,
        }: AudioAnalyzerOption,
        mut progress_callback: F,
    ) -> Result<Vec<Segment>, AudioAnalyzerError>
    where
        F: FnMut(Progress) -> () + Send + Sync + 'static,
    {
        let processed_audio = self.process_audio_samples(audio_path, &mut progress_callback)?;

        Ok(self.find_non_silent_segments(
            processed_audio,
            threshold_db,
            min_duration_ms,
            left_buffer_sec,
            right_buffer_sec,
            &mut progress_callback,
        ))
    }
}
