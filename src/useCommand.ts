import { invoke } from "@tauri-apps/api/core";
import { AnalysisResult, VideoInfo } from "./interface";

const useCommand = () => {
  const getVideoInfo = async (videoPath: string): Promise<VideoInfo> => {
    const videoInfo = await invoke<VideoInfo>("get_video_info", { videoPath });
    return videoInfo;
  };

  const analyzeVideo = async (
    videoPath: string,
    useDeepfilternet: boolean,
    thresholdDb: number,
    minDurationMs: number,
    leftBufferSec: number,
    rightBufferSec: number,
    useNormalize: boolean = false,
    peakNormalization: boolean = false,
    targetDb: number = -3.0,
  ): Promise<AnalysisResult> => {
    const result = await invoke<AnalysisResult>("analyze_video", {
      videoPath,
      useDeepfilternet,
      thresholdDb,
      minDurationMs,
      leftBufferSec,
      rightBufferSec,
      useNormalize,
      peakNormalization,
      targetDb,
    });
    return result;
  };

  const generateFcpXml = async (
    segments: [number, number][],
    videoPath: string,
    fps: string,
    resolution: string,
    outputPath: string,
  ): Promise<string> => {
    const result = await invoke<string>("generate_fcpxml", {
      segments,
      videoPath,
      fps,
      resolution,
      outputPath,
    });
    return result;
  };

  return { getVideoInfo, analyzeVideo, generateFcpXml };
};

export default useCommand;
