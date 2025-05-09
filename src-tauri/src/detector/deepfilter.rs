use std::time::Instant;

use df::{
    tract::{DfParams, DfTract, RuntimeParams},
    transforms::resample,
    wav_utils::{write_wav_arr2, ReadWav},
};
use ndarray::{Array2, ArrayD, Axis};

#[derive(Debug, thiserror::Error)]
pub enum DeepFilterNetError {
    #[error("Input not valid: {0}")]
    InputNotValid(String),
}

pub struct Parameter {
    post_filter: bool,
    post_filter_beta: f32,
    compensate_delay: bool,
    atten_lim_db: f32,
    min_db_thresh: f32,
    max_db_erb_thresh: f32,
    max_db_df_thresh: f32,
    reduce_mask: i32,
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            post_filter: true,
            post_filter_beta: 0.02,
            compensate_delay: true,
            atten_lim_db: 100.0,
            min_db_thresh: -15.0,
            max_db_erb_thresh: 35.0,
            max_db_df_thresh: 35.0,
            reduce_mask: 1,
        }
    }
}

pub fn apply_deepfilternet<F>(
    params: Parameter,
    model_path: &str,
    audio_path: &str,
    output_path: &str,
    mut progress_callback: F,
) -> Result<(), DeepFilterNetError>
where
    F: FnMut(f32) + Send + Sync + 'static,
{
    // Initialize with 1 channel
    let mut r_params = RuntimeParams::default_with_ch(1);
    r_params = r_params
        .with_atten_lim(params.atten_lim_db)
        .with_thresholds(
            params.min_db_thresh,
            params.max_db_erb_thresh,
            params.max_db_df_thresh,
        );
    if params.post_filter {
        r_params = r_params.with_post_filter(params.post_filter_beta);
    }
    if let Ok(red) = params.reduce_mask.try_into() {
        r_params = r_params.with_mask_reduce(red);
    } else {
        return Err(DeepFilterNetError::InputNotValid(
            "invalid reduce_mask".to_string(),
        ));
    }
    let df_params = match DfParams::new(model_path.into()) {
        Ok(df) => df,
        Err(e) => return Err(DeepFilterNetError::InputNotValid(e.to_string())),
    };

    println!("test");
    let mut model: DfTract = DfTract::new(df_params.clone(), &r_params)
        .map_err(|e| DeepFilterNetError::InputNotValid(e.to_string()))?;
    println!("test");
    let mut sr = model.sr;
    let mut delay = model.fft_size - model.hop_size; // STFT delay
    delay += model.lookahead * model.hop_size; // Add model latency due to lookahead

    let reader =
        ReadWav::new(audio_path).map_err(|e| DeepFilterNetError::InputNotValid(e.to_string()))?;

    // Check if we need to adjust to multiple channels
    if r_params.n_ch != reader.channels {
        r_params.n_ch = reader.channels;
        model = DfTract::new(df_params.clone(), &r_params)
            .map_err(|e| DeepFilterNetError::InputNotValid(e.to_string()))?;
        sr = model.sr;
    }
    let sample_sr = reader.sr;
    let mut noisy = reader
        .samples_arr2()
        .map_err(|e| DeepFilterNetError::InputNotValid(e.to_string()))?;

    if sr != sample_sr {
        noisy = resample(noisy.view(), sample_sr, sr, None).expect("Error during resample()");
    }
    let noisy = noisy.as_standard_layout();
    let mut enh: Array2<f32> = ArrayD::default(noisy.shape())
        .into_dimensionality()
        .map_err(|e| DeepFilterNetError::InputNotValid(e.to_string()))?;
    let t0 = Instant::now();

    // 총 처리할 청크 수 계산
    let total_chunks = noisy.len_of(Axis(1)) / model.hop_size;
    let mut progress_counter = 0;

    for (ns_f, enh_f) in noisy
        .view()
        .axis_chunks_iter(Axis(1), model.hop_size)
        .zip(enh.view_mut().axis_chunks_iter_mut(Axis(1), model.hop_size))
    {
        if ns_f.len_of(Axis(1)) < model.hop_size {
            break;
        }

        // 청크 처리
        model
            .process(ns_f, enh_f)
            .map_err(|e| DeepFilterNetError::InputNotValid(e.to_string()))?;

        // 진행률 업데이트 및 표시 (10% 단위로 표시)
        progress_counter += 1;
        let progress_percent = (progress_counter as f32 / total_chunks as f32) * 100.0;

        if progress_counter == 1
            || progress_counter % (total_chunks / 10).max(1) == 0
            || progress_counter == total_chunks
        {
            progress_callback(progress_percent);
        }
    }

    let elapsed = t0.elapsed().as_secs_f32();
    let t_audio = noisy.len_of(Axis(1)) as f32 / sr as f32;
    println!(
        "Enhanced audio file {} in {:.2} (RTF: {})",
        audio_path,
        elapsed,
        elapsed / t_audio
    );

    if params.compensate_delay {
        enh.slice_axis_inplace(Axis(1), ndarray::Slice::from(delay..));
    }
    if sr != sample_sr {
        enh = resample(enh.view(), sr, sample_sr, None).expect("Error during resample()");
    }
    write_wav_arr2(output_path, enh.view(), sample_sr as u32)
        .map_err(|e| DeepFilterNetError::InputNotValid(e.to_string()))?;

    Ok(())
}
