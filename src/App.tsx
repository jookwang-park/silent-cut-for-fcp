import "./App.css";
import { useState, useEffect } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from '@tauri-apps/api/app';
import { useTranslation } from "react-i18next";
import { LanguageSwitcher } from "./components/LanguageSwitcher";
import useCommand from "./useCommand";
import {
  AnalysisResult,
  AnalysisProgress,
  Segment,
  VideoInfo,
} from "./interface";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
  CardFooter,
} from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Progress } from "@/components/ui/progress";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  CheckCircle,
  AlertCircle,
  Film,
  Upload,
  Waves,
  Download,
  Clock,
  Info,
  Settings,
  FileCheck,
  Check,
  Volume2,
  Zap,
} from "lucide-react";

function App() {
  const { t } = useTranslation();
  const [videoPath, setVideoPath] = useState<string>("");
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);
  const [fileName, setFileName] = useState<string>("");
  const [isAnalyzing, setIsAnalyzing] = useState<boolean>(false);
  const [isGenerating, setIsGenerating] = useState<boolean>(false);
  const [segments, setSegments] = useState<Segment[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [progress, setProgress] = useState<AnalysisProgress>({
    phase: "",
    percentage: 0,
  });
  const [useDeepfilternet, setUseDeepfilternet] = useState<boolean>(true);
  const [useNormalize, setUseNormalize] = useState<boolean>(false);
  const [targetDb, setTargetDb] = useState<number>(-3.0);
  const [peakNormalization, setPeakNormalization] = useState<boolean>(false);
  const [version, setVersion] = useState<string>("");

  const { getVideoInfo, analyzeVideo, generateFcpXml } = useCommand();

  // 설정 상태
  const [thresholdDb, setThresholdDb] = useState<number>(10);
  const [minDurationMs, setMinDurationMs] = useState<number>(100);
  const [leftBufferSec, setLeftBufferSec] = useState<number>(0.01);
  const [rightBufferSec, setRightBufferSec] = useState<number>(0.15);
  const [fps, setFps] = useState<string>("auto");
  const [resolution, setResolution] = useState<string>("auto");

  useEffect(() => {
    getVersion().then((version) => setVersion(version));
  }, []);

  // 진행률 이벤트 리스너 설정
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen<AnalysisProgress>("analyze-progress", (event) => {
        setProgress(event.payload);
      });
    };

    setupListener();

    // 컴포넌트 언마운트 시 리스너 정리
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // 파일 선택 처리
  const handleFileSelect = async () => {
    try {
      const homeDirPath = await homeDir();
      const selected = await open({
        directory: false,
        multiple: false,
        defaultPath: homeDirPath,
        filters: [
          {
            name: "비디오 파일",
            extensions: ["mp4", "mov", "avi", "mkv"],
          },
        ],
      });

      if (selected && typeof selected === "string") {
        setVideoPath(selected);

        const pathParts = selected.split(/[/\\]/);
        const fileWithExt = pathParts[pathParts.length - 1];
        const fileName = fileWithExt.split(".").slice(0, -1).join(".");
        setFileName(fileName || "unknown");

        setError(null);
        setSuccess(null);
        setSegments([]);
        setProgress({ phase: "", percentage: 0 });

        // 비디오 정보 가져오기 및 상태 업데이트
        try {
          const info = await getVideoInfo(selected);
          setVideoInfo(info);
          // 자동 설정 적용
          setFps(info.fps > 0 ? info.fps.toFixed(2) : "30");
          if (info.height > 1080) setResolution("4K");
          else if (info.height > 720) setResolution("FHD");
          else if (info.height > 0) setResolution("HD");
          else setResolution("FHD");
        } catch (infoError) {
          setError(t("fileSelect.videoInfo", { message: infoError }));
          setVideoInfo(null);
          setFps("30");
          setResolution("FHD");
        }
      }
    } catch (err) {
      setError(t("fileSelect.fileSelectionError", { message: err }));
      setVideoInfo(null);
    }
  };

  // 비디오 분석 처리
  const handleAnalyze = async () => {
    if (!videoPath) {
      setError(t("alerts.selectVideo"));
      setError("비디오 파일을 먼저 선택해주세요.");
      return;
    }

    setIsAnalyzing(true);
    setError(null);
    setSuccess(null);
    setProgress({ phase: "", percentage: 0 });

    try {
      const result: AnalysisResult = await analyzeVideo(
        videoPath,
        useDeepfilternet,
        thresholdDb,
        minDurationMs,
        leftBufferSec,
        rightBufferSec,
        useNormalize,
        peakNormalization,
        targetDb
      );

      // 결과 변환 및 저장
      setSegments(result.segments);
      setSuccess(
        t("alerts.analysisComplete", {
          count: result.segments.length,
        }),
      );
    } catch (err) {
      setError(t("alerts.analysisError", { error: err }));
    } finally {
      setIsAnalyzing(false);
      setProgress({ phase: "Done", percentage: 100 });
    }
  };

  // FCPXML 생성 처리
  const handleGenerateFCPXML = async () => {
    if (!videoPath || segments.length === 0) {
      setError(t("alerts.selectVideo"));
      return;
    }

    setIsGenerating(true);
    setError(null);

    try {
      // 저장 경로 선택
      const homeDirPath = await homeDir();
      const outputPath = await save({
        defaultPath: `${homeDirPath}/${fileName}_silent_cut.fcpxml`,
        filters: [
          {
            name: "FCPXML 파일",
            extensions: ["fcpxml"],
          },
        ],
      });

      if (!outputPath) {
        setIsGenerating(false);
        return;
      }

      // FCPXML 생성 호출
      const segmentTuples: [number, number][] = segments.map(
        ({ start, end }) => [start, end],
      );
      const actualFps =
        fps === "auto" && videoInfo ? videoInfo.fps.toFixed(2) : fps;
      const actualResolution =
        resolution === "auto" && videoInfo
          ? videoInfo.height > 1080
            ? "4K"
            : videoInfo.height > 720
              ? "FHD"
              : "HD"
          : resolution;

      await generateFcpXml(
        segmentTuples,
        videoPath,
        actualFps,
        actualResolution,
        outputPath,
      );

      setSuccess(t("alerts.fcpxmlGenerated", { path: outputPath }));
    } catch (err) {
      setError(t("alerts.fcpxmlError", { error: err }));
    } finally {
      setIsGenerating(false);
    }
  };

  // 시간 형식 포맷팅 (초 -> 00:00:00.000)
  const formatTime = (seconds: number): string => {
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    const ms = Math.floor((seconds % 1) * 1000);

    return `${hrs.toString().padStart(2, "0")}:${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}.${ms.toString().padStart(3, "0")}`;
  };

  return (
    <TooltipProvider>
      <div className="container mx-auto p-4 md:p-6 max-w-5xl bg-background text-foreground min-h-screen flex flex-col">
        <header className="text-center mb-6 md:mb-8 relative">
          <div className="absolute right-0 top-0">
            <LanguageSwitcher />
          </div>
          <h1 className="text-3xl md:text-4xl font-bold mb-2 bg-gradient-to-r from-primary to-purple-500 text-transparent bg-clip-text">
            {t("app.title")}
            <span className="text-xs text-muted-foreground">
              v{version}
            </span>
          </h1>
          <p className="text-muted-foreground text-lg md:text-xl">
            {t("app.description")}
          </p>
        </header>

        <main className="flex-grow space-y-6">
          <Card className="animate-fade-in-up animation-delay-100">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-lg md:text-xl">
                <Film className="h-5 w-5 text-primary" />
                <span>{t("fileSelect.title")}</span>
              </CardTitle>
              <CardDescription>{t("fileSelect.description")}</CardDescription>
            </CardHeader>
            <CardContent>
              <div
                className="border-2 border-dashed border-border rounded-lg p-6 text-center cursor-pointer hover:border-primary/80 hover:bg-muted/50 transition-all duration-200"
                onClick={handleFileSelect}
              >
                {videoInfo ? (
                  <div className="text-left space-y-2">
                    <div className="flex items-center gap-2 text-primary">
                      <FileCheck className="h-5 w-5" />
                      <p className="font-semibold text-lg">{fileName}</p>
                    </div>
                    <p className="text-sm text-muted-foreground break-all">
                      {videoPath}
                    </p>
                    <div className="flex flex-wrap gap-x-4 gap-y-1 text-sm pt-2">
                      <span>
                        <strong>{t("fileSelect.resolution")}: </strong>{" "}
                        {videoInfo.width}x{videoInfo.height}
                      </span>
                      <span>
                        <strong>{t("fileSelect.fps")}: </strong>{" "}
                        {videoInfo.fps.toFixed(2)}
                      </span>
                    </div>
                  </div>
                ) : (
                  <div className="flex flex-col items-center py-4">
                    <Upload className="h-10 w-10 text-muted-foreground mb-3" />
                    <p className="font-medium text-base">
                      {t("fileSelect.button")}
                    </p>
                    <p className="text-sm text-muted-foreground mt-1">
                      {t("fileSelect.supportedFormats")}
                    </p>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>

          <Card
            className={`transition-opacity duration-500 ${videoPath ? "opacity-100 animate-fade-in-up animation-delay-200" : "opacity-50 pointer-events-none"}`}
          >
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-lg md:text-xl">
                <Settings className="h-5 w-5 text-primary" />
                <span>{t("analysisSettings.title")}</span>
              </CardTitle>
              <CardDescription>
                {t("analysisSettings.description")}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-6">
                <div className="space-y-5">
                  <h3 className="text-base font-semibold mb-3 border-b pb-1">
                    {t("analysisSettings.detectionSettings")}
                  </h3>
                  <div className="space-y-1">
                    <div className="flex justify-between items-center mb-1">
                      <label
                        htmlFor="threshold"
                        className="text-sm font-medium flex items-center gap-1.5"
                      >
                        {t("analysisSettings.thresholdDb.label")}
                        <Tooltip delayDuration={100}>
                          <TooltipTrigger asChild>
                            <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                          </TooltipTrigger>
                          <TooltipContent>
                            <p>{t("analysisSettings.thresholdDb.tooltip")}</p>
                          </TooltipContent>
                        </Tooltip>
                      </label>
                      <span className="text-sm font-semibold tabular-nums">
                        {thresholdDb} dB
                      </span>
                    </div>
                    <Slider
                      id="threshold"
                      min={-60}
                      max={60}
                      step={1}
                      value={[thresholdDb]}
                      onValueChange={(value) => setThresholdDb(value[0])}
                      className="w-full"
                      disabled={!videoPath || isAnalyzing}
                    />
                  </div>

                  <div className="space-y-1">
                    <div className="flex justify-between items-center mb-1">
                      <label
                        htmlFor="minDuration"
                        className="text-sm font-medium flex items-center gap-1.5"
                      >
                        {t("analysisSettings.minDuration.label")}
                        <Tooltip delayDuration={100}>
                          <TooltipTrigger asChild>
                            <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                          </TooltipTrigger>
                          <TooltipContent>
                            <p>{t("analysisSettings.minDuration.tooltip")}</p>
                          </TooltipContent>
                        </Tooltip>
                      </label>
                      <span className="text-sm font-semibold tabular-nums">
                        {minDurationMs} ms
                      </span>
                    </div>
                    <Slider
                      id="minDuration"
                      min={10}
                      max={1000}
                      step={10}
                      value={[minDurationMs]}
                      onValueChange={(value) => setMinDurationMs(value[0])}
                      className="w-full"
                      disabled={!videoPath || isAnalyzing}
                    />
                  </div>

                  <div className="space-y-1">
                    <div className="flex justify-between items-center mb-1">
                      <label
                        htmlFor="leftBufferSec"
                        className="text-sm font-medium flex items-center gap-1.5"
                      >
                        {t("analysisSettings.leftBufferSec.label")}
                        <Tooltip delayDuration={100}>
                          <TooltipTrigger asChild>
                            <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                          </TooltipTrigger>
                          <TooltipContent>
                            <p>{t("analysisSettings.leftBufferSec.tooltip")}</p>
                          </TooltipContent>
                        </Tooltip>
                      </label>
                      <span className="text-sm font-semibold tabular-nums">
                        {leftBufferSec.toFixed(2)} sec
                      </span>
                    </div>
                    <Slider
                      id="leftBufferSec"
                      min={0}
                      max={1.0}
                      step={0.01}
                      value={[leftBufferSec]}
                      onValueChange={(value) => setLeftBufferSec(value[0])}
                      className="w-full"
                      disabled={!videoPath || isAnalyzing}
                    />
                  </div>
                  <div className="space-y-1">
                    <div className="flex justify-between items-center mb-1">
                      <label
                        htmlFor="rightBufferSec"
                        className="text-sm font-medium flex items-center gap-1.5"
                      >
                        {t("analysisSettings.rightBufferSec.label")}
                        <Tooltip delayDuration={100}>
                          <TooltipTrigger asChild>
                            <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                          </TooltipTrigger>
                          <TooltipContent>
                            <p>{t("analysisSettings.buffer.tooltip")}</p>
                          </TooltipContent>
                        </Tooltip>
                      </label>
                      <span className="text-sm font-semibold tabular-nums">
                        {rightBufferSec.toFixed(2)} sec
                      </span>
                    </div>
                    <Slider
                      id="rightBufferSec"
                      min={0}
                      max={1.0}
                      step={0.01}
                      value={[rightBufferSec]}
                      onValueChange={(value) => setRightBufferSec(value[0])}
                      className="w-full"
                      disabled={!videoPath || isAnalyzing}
                    />
                  </div>
                </div>

                <div className="space-y-5">
                  <h3 className="text-base font-semibold mb-3 border-b pb-1">
                    {t("analysisSettings.outputSettings")}
                  </h3>
                  <div className="space-y-1">
                    <label
                      htmlFor="fps-select"
                      className="text-sm font-medium flex items-center gap-1.5"
                    >
                      {t("analysisSettings.fps.label")}
                      <Tooltip delayDuration={100}>
                        <TooltipTrigger asChild>
                          <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{t("analysisSettings.fps.tooltip")}</p>
                        </TooltipContent>
                      </Tooltip>
                    </label>
                    <Select
                      value={fps}
                      onValueChange={setFps}
                      disabled={!videoPath || isAnalyzing}
                    >
                      <SelectTrigger id="fps-select" className="w-full">
                        <SelectValue placeholder="FPS 선택..." />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="auto">
                          {t("analysisSettings.fps.auto")} (
                          {videoInfo ? videoInfo.fps.toFixed(2) : "N/A"})
                        </SelectItem>
                        <SelectItem value="29.97">29.97 fps</SelectItem>
                        <SelectItem value="30.00">30 fps</SelectItem>
                        <SelectItem value="59.94">59.94 fps</SelectItem>
                        <SelectItem value="60.00">60 fps</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="space-y-1">
                    <label
                      htmlFor="resolution-select"
                      className="text-sm font-medium flex items-center gap-1.5"
                    >
                      {t("analysisSettings.resolution.label")}
                      <Tooltip delayDuration={100}>
                        <TooltipTrigger asChild>
                          <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{t("analysisSettings.resolution.tooltip")}</p>
                        </TooltipContent>
                      </Tooltip>
                    </label>
                    <Select
                      value={resolution}
                      onValueChange={setResolution}
                      disabled={!videoPath || isAnalyzing}
                    >
                      <SelectTrigger id="resolution-select" className="w-full">
                        <SelectValue placeholder="해상도 선택..." />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="auto">
                          {t("analysisSettings.resolution.auto")} (
                          {videoInfo
                            ? `${videoInfo.width}x${videoInfo.height}`
                            : "N/A"}
                          )
                        </SelectItem>
                        <SelectItem value="SD">SD (720x480)</SelectItem>
                        <SelectItem value="HD">HD (1280x720)</SelectItem>
                        <SelectItem value="FHD">FHD (1920x1080)</SelectItem>
                        <SelectItem value="4K">4K (3840x2160)</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
              </div>
              <div className="mt-4 p-3">
                <div className="mt-4 p-3 bg-accent/40 rounded-md border border-border">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <Zap className="h-5 w-5 text-primary" />
                      <h4 className="text-sm font-semibold">{t("analysisSettings.advancedAudio.title")}</h4>
                    </div>
                    <Button
                      onClick={() => setUseDeepfilternet(!useDeepfilternet)}
                      className={`h-8 px-3 flex items-center gap-2 ${useDeepfilternet
                        ? "bg-primary text-primary-foreground"
                        : "bg-secondary text-secondary-foreground border border-input"
                        }`}
                      disabled={!videoPath || isAnalyzing}
                    >
                      {useDeepfilternet ? <Check className="h-4 w-4" /> : null}
                      <span className="text-sm">{t(`analysisSettings.advancedAudio.deepFilterNet.${useDeepfilternet ? "enabled" : "disabled"}`)}</span>
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {t("analysisSettings.advancedAudio.deepFilterNet.description")}
                  </p>
                </div>

                <div className="mt-4 p-3 bg-accent/40 rounded-md border border-border">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <Volume2 className="h-5 w-5 text-primary" />
                      <h4 className="text-sm font-semibold">{t("analysisSettings.advancedAudio.normalization.title")}</h4>
                    </div>
                    <Button
                      onClick={() => setUseNormalize(!useNormalize)}
                      className={`h-8 px-3 flex items-center gap-2 ${useNormalize
                        ? "bg-primary text-primary-foreground"
                        : "bg-secondary text-secondary-foreground border border-input"
                        }`}
                      disabled={!videoPath || isAnalyzing}
                    >
                      {useNormalize ? <Check className="h-4 w-4" /> : null}
                      <span className="text-sm">{t(`analysisSettings.advancedAudio.normalization.${useNormalize ? "enabled" : "disabled"}`)}</span>
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground mb-3">
                    {t("analysisSettings.advancedAudio.normalization.description")}
                  </p>

                  {useNormalize && (
                    <div className="space-y-3 pl-2 border-l-2 border-primary/20 mt-3">
                      <div className="space-y-1">
                        <div className="flex justify-between items-center mb-1">
                          <label
                            htmlFor="targetDb"
                            className="text-sm font-medium flex items-center gap-1.5"
                          >
                            {t("analysisSettings.advancedAudio.normalization.targetLevel")}
                            <Tooltip delayDuration={100}>
                              <TooltipTrigger asChild>
                                <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                              </TooltipTrigger>
                              <TooltipContent>
                                <p>{t("analysisSettings.advancedAudio.normalization.targetLevelTooltip")}</p>
                              </TooltipContent>
                            </Tooltip>
                          </label>
                          <span className="text-sm font-semibold tabular-nums">
                            {targetDb} dB
                          </span>
                        </div>
                        <Slider
                          id="targetDb"
                          min={-24}
                          max={0}
                          step={0.5}
                          value={[targetDb]}
                          onValueChange={(value) => setTargetDb(value[0])}
                          className="w-full"
                          disabled={!videoPath || isAnalyzing || !useNormalize}
                        />
                      </div>

                      <div className="flex items-center justify-between pt-1">
                        <div className="flex items-center gap-2">
                          <span className="text-sm">{t("analysisSettings.advancedAudio.normalization.method")}</span>
                        </div>
                        <div className="flex gap-2">
                          <Button
                            onClick={() => setPeakNormalization(true)}
                            className={`h-8 px-3 flex items-center gap-1.5 border ${peakNormalization
                              ? "bg-primary text-primary-foreground border-primary font-medium"
                              : "bg-background hover:bg-accent border-input"
                              }`}
                            disabled={!videoPath || isAnalyzing || !useNormalize}
                          >
                            {peakNormalization && <Check className="h-3.5 w-3.5" />}
                            <span>{t("analysisSettings.advancedAudio.normalization.peakBased")}</span>
                          </Button>
                          <Button
                            onClick={() => setPeakNormalization(false)}
                            className={`h-8 px-3 flex items-center gap-1.5 border ${!peakNormalization
                              ? "bg-primary text-primary-foreground border-primary font-medium"
                              : "bg-background hover:bg-accent border-input"
                              }`}
                            disabled={!videoPath || isAnalyzing || !useNormalize}
                          >
                            {!peakNormalization && <Check className="h-3.5 w-3.5" />}
                            <span>{t("analysisSettings.advancedAudio.normalization.rmsBased")}</span>
                          </Button>
                        </div>
                      </div>
                      <p className="text-xs text-muted-foreground mt-1">
                        {peakNormalization
                          ? t("analysisSettings.advancedAudio.normalization.peakDescription")
                          : t("analysisSettings.advancedAudio.normalization.rmsDescription")}
                      </p>
                    </div>
                  )}
                </div>
              </div>
            </CardContent>
            <CardFooter className="flex flex-col items-stretch gap-4">
              <div
                className={`transition-all duration-300 ease-out ${isAnalyzing ? "h-auto opacity-100" : "h-0 opacity-0"} overflow-hidden`}
              >
                <p className="text-xs text-muted-foreground text-center mt-1">
                  {progress.phase}
                </p>
                <Progress value={progress.percentage} className="w-full h-2" />
                <p className="text-xs text-muted-foreground text-center mt-1">
                  {progress.percentage.toFixed(2)}%
                </p>
              </div>
              <Button
                onClick={handleAnalyze}
                disabled={!videoPath || isAnalyzing}
                className="w-full"
                size="lg"
              >
                <Waves className="mr-2 h-4 w-4" />
                {isAnalyzing
                  ? t("analysisSettings.analyzingButton")
                  : t("analysisSettings.analyzeButton")}
              </Button>
            </CardFooter>
          </Card>

          {error && (
            <Alert variant="destructive" className="animate-fade-in">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>{t("alerts.error")}</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {success && !error && (
            <Alert className="bg-green-50 dark:bg-green-900/30 text-green-800 dark:text-green-300 border-green-200 dark:border-green-700 animate-fade-in">
              <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
              <AlertTitle>{t("alerts.success")}</AlertTitle>
              <AlertDescription>{success}</AlertDescription>
            </Alert>
          )}

          {segments.length > 0 && (
            <Card className="animate-fade-in-up animation-delay-300">
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-lg md:text-xl">
                  <Clock className="h-5 w-5 text-primary" />
                  <span>{t("analysisResult.title")}</span>
                </CardTitle>
                <CardDescription>
                  {t("analysisResult.description", { count: segments.length })}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="border border-border rounded-lg max-h-[300px] md:max-h-[400px] overflow-y-auto shadow-inner">
                  <ul className="divide-y divide-border">
                    {segments.map((segment, index) => (
                      <li
                        key={index}
                        className="flex flex-col sm:flex-row justify-between items-start sm:items-center p-3 hover:bg-muted/50 text-sm"
                      >
                        <div className="flex items-center gap-2 mb-1 sm:mb-0">
                          <span className="font-mono text-xs bg-secondary text-secondary-foreground rounded px-1.5 py-0.5 w-8 text-center">
                            {index + 1}
                          </span>
                          <span className="font-medium tabular-nums">
                            {formatTime(segment.start)} -{" "}
                            {formatTime(segment.end)}
                          </span>
                        </div>
                        <span className="text-xs bg-primary/10 text-primary font-medium px-2 py-0.5 rounded-full self-end sm:self-center tabular-nums">
                          {(segment.end - segment.start).toFixed(2)}
                          {t("analysisResult.seconds")}
                        </span>
                      </li>
                    ))}
                  </ul>
                </div>
              </CardContent>
              <CardFooter className="flex flex-col items-stretch gap-4">
                <Button
                  onClick={handleGenerateFCPXML}
                  disabled={isGenerating || segments.length === 0}
                  className="w-full"
                  size="lg"
                >
                  <Download className="mr-2 h-4 w-4" />
                  {isGenerating
                    ? t("analysisResult.generatingButton")
                    : t("analysisResult.generateButton")}
                </Button>
              </CardFooter>
            </Card>
          )}
        </main>

        <footer className="mt-12 pt-6 border-t border-border/40 text-center">
          <div className="flex flex-col md:flex-row justify-between items-center gap-4 max-w-3xl mx-auto px-4">
            <div className="flex flex-col items-center md:items-start">
              <p className="font-medium text-sm bg-gradient-to-r from-primary to-purple-500 text-transparent bg-clip-text">
                Silent Cut for FCP
              </p>
            </div>

            <div className="flex items-center gap-6">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <a
                      href="https://github.com/jookwang-park/silent-cut-for-fcp"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-muted-foreground hover:text-primary transition-colors duration-200"
                      aria-label="GitHub"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="w-5 h-5">
                        <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
                      </svg>
                    </a>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{t("footer.github")}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <a
                      href="https://www.youtube.com/@parks-garage-kr"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-muted-foreground hover:text-red-500 transition-colors duration-200"
                      aria-label="YouTube"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="w-5 h-5">
                        <path d="M22.54 6.42a2.78 2.78 0 0 0-1.94-2C18.88 4 12 4 12 4s-6.88 0-8.6.46a2.78 2.78 0 0 0-1.94 2A29 29 0 0 0 1 11.75a29 29 0 0 0 .46 5.33A2.78 2.78 0 0 0 3.4 19c1.72.46 8.6.46 8.6.46s6.88 0 8.6-.46a2.78 2.78 0 0 0 1.94-2 29 29 0 0 0 .46-5.25 29 29 0 0 0-.46-5.33z" />
                        <polygon points="9.75 15.02 15.5 11.75 9.75 8.48 9.75 15.02" />
                      </svg>
                    </a>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{t("footer.youtube")}</p>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>
        </footer>
      </div>
    </TooltipProvider>
  );
}

export default App;
