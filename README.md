# Silent Cut for FCP

Silent Cut for FCP는 비디오 파일에서 소리가 있는 부분만 자동으로 감지하여
Final Cut Pro에서 사용할 수 있는 FCPXML 파일로 변환해주는 데스크톱 애플리케이션입니다.

## 제작자의 말

본 프로그램은 Park's Garage가 만들었습니다. 파이널컷 프로를 사용하고 계시는 분들께서,
좀 더 빠르고 효율적으로 영상 편집을 하는데 도움이 되었으면 좋겠습니다.

제작 과정은 다음 유튜브 영상을 참고해주세요. 이왕이면 구독과 좋아요도 눌러주세요! 

👉🏻 [업데이트 후 반영예정](https://youtube.com)

## 주요 기능

- 비디오 파일에서 오디오 분석 및 소리가 있는 구간 자동 감지
- 감지된 구간을 Final Cut Pro 호환 FCPXML 파일로 내보내기
- 다양한 FPS 및 해상도 설정 지원

## 기술 스택

- **백엔드**: Rust (Tauri)
  - Symphonia: 오디오 디코딩 및 분석
  - ez-ffmpeg: 비디오에서 오디오 추출
  - xml-builder: FCPXML 파일 생성
- **프론트엔드**: React + TypeScript
  - Tauri API를 통한 백엔드 연동

## 시작하기

### 필수 조건

- [Node.js](https://nodejs.org/) (v18 이상)
- [Rust](https://www.rust-lang.org/tools/install) (최신 버전)
- [FFmpeg](https://ffmpeg.org/download.html) (시스템에 설치 필요)

### ffmpeg 설치 (macOS)

```bash
brew install ffmpeg
```

### 개발 환경 설정

1. 저장소 복제
   ```bash
   git clone https://github.com/jookwang-park/silent-cut-for-fcp.git
   cd silent-cut-for-fcp
   ```

2. 의존성 설치
   ```bash
   pnpm install
   ```

3. 개발 모드로 실행
   ```bash
   pnpm tauri dev
   ```

## 라이선스

[MIT](LICENSE)

