
export interface VideoInfo {
    duration: number;
    width: number;
    height: number;
    fps: number;
}

export interface Segment {
    start: number;
    end: number;
}

export interface AnalysisResult {
    segments: Segment[];
    outputPath: string;
}

export interface AnalysisProgress {
    phase: string;
    percentage: number;
}
