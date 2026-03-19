# mig-gpu-mon

**한국어** | [English](README_EN.md)

NVIDIA MIG(Multi-Instance GPU) 환경에서 `nvidia-smi`가 제공하지 못하는 GPU 메트릭을 실시간 모니터링하는 터미널 TUI 프로그램.

btop/nvtop 스타일의 실시간 sparkline 그래프를 터미널에서 표시하며, CPU 코어별 사용률과 시스템 RAM도 함께 모니터링한다.

> **Ubuntu 특화:** 개발 및 테스트가 Ubuntu 환경에서 이루어졌으며, 라이브러리 탐색 경로·에러 메시지·문서 모두 Ubuntu를 1차 대상으로 작성되었습니다. RHEL 계열, 컨테이너, WSL2 등 다른 환경에서도 동작하지만, Ubuntu에서 가장 매끄럽게 동작합니다.

## Screen Layout

### ASCII 구성도

```
┌─ mig-gpu-mon ────────────────────────────────── 2026-03-17 02:15:30 PM ┐
│ MIG GPU Monitor | Driver: 535.129.03 | CUDA: 12.2 | GPUs: 3           │ ← Header
├─ CPU (64 cores) 23.4% ─────────┬─ Devices ────────────────────────────┤
│ 17 ▮▮▮▮▮▮▮  92%   5 ▮▮▮▯▯ 34% │ > MIG 0 (GPU 0: A100) GPU:45% Mem:… │ ↑ 20%
│  2 ▮▮▮▮▮▯▯  65%  40 ▮▮▯▯▯ 18% │   MIG 1 (GPU 0: A100) GPU:12% Mem:… │ ↓
│  0 ▮▮▮▮▯▯▯  52%  33 ▮▯▯▯▯  5% ├─ Detail ─────────────────────────────┤    ← Top 45%
│  ...                            │ Name: MIG 0 (GPU 0: A100-SXM4-80GB) │ ↑
├─ Memory used/cached/free avl ─┤ UUID: MIG-a1b2...  Arch:Ampere CC:8.0│ │
│ RAM ▮▮▮▮▮▯▯ 70.1/12.5/6.6 … │ VRAM 12288 MB / 20480 MB (60.0%)    │ │
│ SWP ▮▯▯▯▯▯▯  2.1/32.0 GiB  … │ GPU: 45%  Mem: 38%  SM: 45%         │ │ 50%
│                                 │ Enc: 0%  Dec: 0%                     │ │
│                                 │ Clk: 1410/1410/1215 MHz  P0          │ │
│                                 │ Temp: 62°C (↓90 ✕92)  Power:127/300W│ │
│                                 │ PCIe: Gen4 x16  TX:12.3 RX:56.7 MB/s│ │
│                                 │ ECC: On  Corr:0  Uncorr:0            │ │
│                                 │ Throttle: None   Processes: 2        │ ↓
│                                 ├─ Top Processes ──────────────────────┤
│                                 │ PID     Process         VRAM        │ ↑
│                                 │ 12345   python3          8192 MB    │ │ 30%
│                                 │ 12400   pt_main_thread   4096 MB    │ ↓
├─ GPU Util 45% ──────────────────┬─ CPU Total 23.4% ───────────────────┤
│ ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅            │ ▂▂▃▃▂▂▃▂▃▃▂▂▃▃▂▃                   │ ← 25%
├─ Mem Ctrl 38% ──────────────────┼─ RAM 89.2/256.0 GiB (34.8%) ────────┤    ← Bottom 55%
│ ▃▃▃▄▄▅▅▅▄▃▃▃▄▄▅▅▄             │ ▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅                   │ ← 25%
├─ VRAM 12288/20480 MB (60.0%) ──┼──────────────────────────────────────┤
│ ▅▅▅▅▆▆▆▆▆▆▇▇▇▇▇▇▇             │                                     │ ← 25%
├─ PCIe TX:12.3 RX:56.7 MB/s ────┼──────────────────────────────────────┤
│ ▂▃▃▅▅▆▅▃▂▂▃▅▆▆▅▃              │                                     │ ← 25% (PCIe 있을 때)
├────────────────────────────────────────────────────────────────────────┤
│ q Quit  Tab/↑↓ Switch GPU  [1/3]                                      │ ← Footer
└────────────────────────────────────────────────────────────────────────┘
```

### 레이아웃 계층 구조

코드(`dashboard.rs`)의 실제 레이아웃 트리. 비율은 `Constraint` 값 그대로.

```
draw()
├── Header                          Length(3)
├── Main                            Min(10)
│   ├── [Top 45%]  ─── Horizontal ──────────────────────────
│   │   ├── System Panel  50%
│   │   │   ├── CPU Cores         Min(4)    " CPU ({N} cores) {pct}% "
│   │   │   │   └── dynamic N-column bars   "{idx} ▮▮▯▯ {pct}%" (사용량 높은 순 정렬)
│   │   │   └── RAM / Swap        Length(4)  " Memory used/cached/free avl "
│   │   │       ├── RAM line                 "RAM ▮▮▮▮▯▯ {used}/{cached}/{free} avl:{avail}/{total}G"
│   │   │       │   └── segmented bar: used(Green/Yellow/Red) + cached(Blue) + free(DarkGray)
│   │   │       │       numeric labels: used(color) / cached(Blue) / free(DarkGray) avl(White) / totalG(White)
│   │   │       └── SWP line                 "SWP ▮▮▯▯ {used}/{total} GiB ({pct}%)"
│   │   └── GPU Panel     50%
│   │       ├── Device List        20%       " Devices "
│   │       │   └── "{>} {MIG|GPU} {idx}: {name} | GPU:{pct}% Mem:{pct}%"
│   │       ├── GPU Detail         50%       " Detail "
│   │       │   ├── Name:      {name} [Parent: GPU {n}]   (MIG일 때)
│   │       │   ├── UUID:      {uuid}  Arch:{arch}  CC:{major.minor}
│   │       │   ├── VRAM      {used} MB / {total} MB ({pct}%)
│   │       │   ├── GPU: {pct}%  Mem: {pct}%  SM: {pct}%  (가로 압축)
│   │       │   ├── Enc: {pct}%  Dec: {pct}%
│   │       │   ├── Clk: {gfx}/{sm}/{mem} MHz  {PState}
│   │       │   ├── Temp: {val}°C (↓{slowdown} ✕{shutdown})  Power: {u}/{l}W
│   │       │   ├── PCIe: Gen{n} x{w}  TX:{mb} RX:{mb} MB/s
│   │       │   ├── ECC: On/Off  Corr:{n}  Uncorr:{n}
│   │       │   ├── Throttle: None / {reasons}
│   │       │   └── Processes: {count}
│   │       └── Top Processes      30%       " Top Processes "
│   │           ├── Header: PID / Process / VRAM
│   │           └── {pid} {name (max 15)} {vram} MB  (top 5 by VRAM desc)
│   └── [Bottom 55%] ─── Horizontal ────────────────────────
│       ├── GPU Charts    50%
│       │   ├── GPU Util {pct}%        sparkline   25% (PCIe 있을 때) / 33%
│       │   ├── Mem Ctrl {pct}%        sparkline   25% / 33%
│       │   ├── VRAM {u}/{t} MB ({p}%) sparkline   25% / 34%
│       │   └── PCIe TX/RX MB/s       sparkline   25% (PCIe 데이터 있을 때만)
│       └── System Charts  50%
│           ├── CPU Total {pct}%       sparkline   50%
│           └── RAM {u}/{t} GiB ({p}%) sparkline   50%
└── Footer                          Length(3)
```

### 색상 코딩

| 요소 | 색상 | 조건 |
|------|------|------|
| CPU 코어 바 | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| RAM 바 (Used 구간) | Green / Yellow / Red | 0-50% / 50-80% / 80%+ (전체 사용률 기준) |
| RAM 바 (Cached 구간) | Blue | 커널 캐시/버퍼 (available - free) |
| RAM 바 (Free 구간) | DarkGray | 완전 미사용 |
| RAM 수치 (avl) | White | available (스왑 없이 사용 가능한 양) |
| Swap 바 | DarkGray / Yellow / Red | 0-20% / 20-50% / 50%+ |
| GPU Util sparkline | Green | — |
| Mem Ctrl sparkline | Blue | — |
| VRAM sparkline | Magenta | — |
| PCIe sparkline | LightCyan | PCIe 데이터 있을 때만 표시 |
| CPU sparkline | Cyan | — |
| RAM sparkline | Yellow | — |
| VRAM % (Detail) | Green / Yellow / Red | 0-70% / 70-90% / 90%+ |
| Temp | Green / Yellow / Red | 0-60°C / 60-80°C / 80°C+ |
| Clock 값 | Cyan | — |
| PState | Green / Yellow / Red | P0 / P1-P4 / P5+ |
| PCIe 정보 | LightCyan | — |
| Encoder/Decoder | Magenta | — |
| Throttle "None" | Green | 정상 |
| Throttle 활성 | Red + Bold | 쓰로틀 경고 |
| ECC errors 0 | Green | 정상 |
| ECC uncorrected > 0 | Red + Bold | 위험 |
| 선택된 GPU | Green + Bold | — |
| Header | Cyan + Bold | — |

## Why

MIG 환경에서 `nvidia-smi`는 GPU Utilization, Memory Utilization 등 핵심 메트릭을 표시하지 못한다.
`nvmlDeviceGetUtilizationRates()`가 MIG 디바이스 핸들에서 `NVML_ERROR_NOT_SUPPORTED`를 반환하기 때문이다.

이 도구는 NVML C API를 직접 호출하는 **3단계 폴백 메커니즘**으로 이 제한을 우회한다:

1. **1단계:** `nvmlDeviceGetUtilizationRates()` — 표준 API (비-MIG GPU에서 동작)
2. **2단계:** `nvmlDeviceGetProcessUtilization()` — 프로세스별 SM/Memory utilization 수집 후 집계
3. **3단계:** `nvmlDeviceGetSamples(GPU_UTILIZATION_SAMPLES)` — 부모 GPU 샘플링 + MIG 슬라이스 비율 스케일링

모든 utilization API가 실패하면 (드라이버 535.x MIG에서 흔한 상황) 오해를 유발하는 0% 대신 "N/A"를 표시한다.

## Features

- MIG 인스턴스별 GPU Util, Mem Ctrl(메모리 컨트롤러 / Hopper+에서 DRAM BW Util via GPM), SM Util, VRAM 사용량 실시간 표시
- **Top Processes** — PID, 프로세스명, VRAM 사용량(MB) 기준 상위 5개 표시
- 부모 GPU 메트릭(온도, 전력, 프로세스 수) 동시 표시
- **Clock Speeds** — Graphics/SM/Memory 클럭 (MHz) + Performance State (P0~P15)
- **PCIe Throughput** — Gen/Width + TX/RX 전송률 (MB/s), 조건부 sparkline 그래프
- **Encoder/Decoder Utilization** — NVENC/NVDEC 사용률 (%)
- **ECC 상태** — 활성 여부 + Corrected/Uncorrected 에러 카운트
- **Temperature Thresholds** — Slowdown/Shutdown 임계 온도 표시
- **Throttle Reasons** — GPU 쓰로틀 원인 실시간 표시 (PwrCap, HW-Therm 등)
- **Architecture & Compute Capability** — GPU 아키텍처 (Ampere, Hopper 등) + CUDA CC
- CPU 코어별 사용률 (사용량 높은 순 정렬, 터미널 너비에 따라 동적 멀티컬럼 바 그래프)
- 시스템 RAM (세그먼트 바: used/cached/free 색상 구분 + 각 수치 색상별 표시 + available/total) / Swap 사용량
- GPU Util / Mem Ctrl / **VRAM** / **PCIe** / CPU Total / RAM 시계열 sparkline 그래프 (타이틀에 현재값 표시)
- Tab/방향키로 GPU/MIG 인스턴스 전환
- 단일 바이너리 배포 (~1.5MB, libc 동적 링크 — 별도 런타임 설치 불필요)

### MIG 환경 메트릭 가용성

MIG 인스턴스에서는 일부 메트릭이 Parent GPU에서만 제공된다:

| 메트릭 | MIG 인스턴스 | Parent GPU | Cloud vGPU |
|--------|-------------|-----------|-----------|
| GPU/Mem/SM Util | O (폴백) | O | O |
| VRAM | O | O | O |
| Architecture, CC | O | O | O |
| Clock Speeds | N/A | O | O |
| PCIe Throughput | N/A | O | 제한적 |
| Performance State | N/A | O | O |
| Temperature, Power | N/A | O | O |
| Temp Thresholds | N/A | O | O |
| ECC 상태/에러 | N/A | O | 제한적 |
| Throttle Reasons | N/A | O | 제한적 |
| Encoder/Decoder | N/A | O | O |

## Requirements

- NVIDIA GPU + 드라이버 설치됨
- `libnvidia-ml.so.1` 접근 가능 (드라이버 설치 시 포함)
- 컨테이너 환경: `--gpus all` 또는 nvidia-docker 사용

### NVML 라이브러리 탐색 경로

프로그램 시작 시 다음 경로를 순서대로 탐색하여 NVML 라이브러리를 로딩한다.
`LD_LIBRARY_PATH`에 등록되지 않은 환경(컨테이너, WSL, 비표준 설치 경로)에서도 자동으로 라이브러리를 찾는다.

| 순서 | 경로 | 대상 환경 |
|------|------|-----------|
| 0 | `--nvml-path` 인자 | 사용자 지정 (최우선) |
| 0+ | `LD_LIBRARY_PATH` 내 경로 | 환경변수 기반 (클라우드 커스텀 설정) |
| 1 | `libnvidia-ml.so.1` | 동적 링커 (표준) |
| 2 | `libnvidia-ml.so` | 기본 (심볼릭 링크) |
| 3 | `/usr/lib/x86_64-linux-gnu/libnvidia-ml.so.1` | Debian / Ubuntu (x86_64) |
| 4 | `/usr/lib64/libnvidia-ml.so.1` | RHEL / CentOS / Rocky / Amazon Linux |
| 5 | `/usr/lib/aarch64-linux-gnu/libnvidia-ml.so.1` | Debian / Ubuntu (ARM64, Graviton) |
| 6 | `/usr/local/nvidia/lib64/libnvidia-ml.so.1` | NVIDIA Container Toolkit (vast.io, RunPod, EKS, GKE, AKS) |
| 7 | `/usr/local/nvidia/lib/libnvidia-ml.so.1` | NVIDIA Container Toolkit (대체 경로) |
| 8 | `/run/nvidia/driver/usr/lib/x86_64-linux-gnu/libnvidia-ml.so.1` | NVIDIA GPU Operator (Kubernetes) |
| 9 | `/run/nvidia/driver/usr/lib64/libnvidia-ml.so.1` | NVIDIA GPU Operator (Kubernetes, RHEL) |
| 10 | `/usr/local/cuda/lib64/stubs/libnvidia-ml.so` | CUDA stubs (빌드 전용) |
| 11 | `/usr/lib/wsl/lib/libnvidia-ml.so.1` | WSL2 |

### 환경별 실행 가이드

| 환경 | 실행 방법 |
|------|-----------|
| **Native (Ubuntu/RHEL)** | `mig-gpu-mon` (드라이버 설치 시 즉시 동작) |
| **Docker** | `docker run --gpus all ...` 또는 `--runtime=nvidia -e NVIDIA_DRIVER_CAPABILITIES=compute,utility` |
| **AWS (EC2 p4d/p5, EKS)** | Deep Learning AMI: 즉시 동작. EKS: nvidia-device-plugin 설치 필요 |
| **GCP (a2/a3 VM, GKE)** | GPU VM: 즉시 동작. GKE: nvidia-driver-installer DaemonSet 필요 |
| **vast.io** | 컨테이너에 자동 마운트, 즉시 동작 |
| **RunPod** | 컨테이너에 자동 마운트, 즉시 동작 |
| **Lambda Labs** | 즉시 동작 (네이티브 드라이버 설치) |
| **WSL2** | `wsl --install` 후 Windows NVIDIA 드라이버 설치 필요 |

### WSL2 설정 가이드

**전제 조건:**
- Windows 11 (또는 Windows 10 21H2+)
- WSL2 (WSL1은 GPU 미지원)
- Windows용 NVIDIA 드라이버 (Linux용 아님)

**설치 확인:**
1. PowerShell에서: `wsl -l -v` → VERSION이 2인지 확인
2. WSL 내에서: `nvidia-smi` → GPU 정보 표시되는지 확인
3. WSL 내에서: `ls /usr/lib/wsl/lib/libnvidia-ml.so.1` → 파일 존재 확인

**트러블슈팅:**
- `nvidia-smi` 안 됨 → Windows NVIDIA 드라이버 업데이트
- WSL1 사용 중 → `wsl --set-version <distro> 2`로 변환
- 라이브러리 없음 → Windows NVIDIA 드라이버 재설치

자동 탐지가 실패할 경우 수동 지정:
```bash
mig-gpu-mon --nvml-path /custom/path/libnvidia-ml.so.1
```

## Quick Start (처음부터 끝까지)

새 서버에서 Rust 설치부터 실행까지 **자동 설치 스크립트** 한 번이면 끝:

```bash
# git이 없으면 먼저 설치 (Ubuntu: sudo apt install git / Rocky: sudo dnf install git)
git clone https://github.com/pathcosmos/mig-gpu-mon.git
cd mig-gpu-mon
./install.sh
```

`install.sh`가 자동으로 처리하는 것:
1. `sudo` 사용 가능 여부 확인 (root가 아니고 sudo 없으면 안내 후 중단)
2. `curl` 미설치 시 → 자동 설치 (apt/dnf/yum 자동 판별)
3. `gcc` (C 링커) 미설치 시 → `build-essential`(Ubuntu) 또는 `gcc`(Rocky/RHEL) 자동 설치
4. `git` 미설치 시 → 자동 설치
5. Rust 미설치 시 → rustup으로 자동 설치
6. `cargo build --release` → 최적화 빌드 (LTO + strip, ~1.5MB)
7. 바이너리 복사 (`~/.cargo/bin` → `/usr/local/bin` → `~/.local/bin` 순으로 탐색) + PATH 확인

> Ubuntu, Rocky Linux, CentOS, RHEL, Amazon Linux 모두 대응. 패키지 매니저(apt/dnf/yum)를 자동 감지한다.

설치 완료 후 바로 실행:
```bash
mig-gpu-mon
```

### 수동 설치 (단계별)

```bash
# 0. 빌드 의존성 (Ubuntu)
sudo apt install -y curl git build-essential
# 0. 빌드 의존성 (Rocky/RHEL)
# sudo dnf install -y curl git gcc

# 1. Rust 설치 (이미 설치되어 있으면 생략)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. 소스 다운로드
git clone https://github.com/pathcosmos/mig-gpu-mon.git
cd mig-gpu-mon

# 3. 빌드 + 시스템 등록 (한 줄)
cargo install --path .

# 4. 실행
mig-gpu-mon
```

`cargo install`은 릴리즈 빌드(LTO + strip) 후 `~/.cargo/bin/mig-gpu-mon`에 자동 등록한다.
`~/.cargo/bin`이 `PATH`에 포함되어 있으므로 어디서든 `mig-gpu-mon`으로 실행 가능.

### 원라인 설치 (복사-붙여넣기용)

> **전제:** `curl`, `git`, `gcc`가 설치되어 있어야 한다. 없으면 위의 수동 설치 0번 단계를 먼저 실행.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source "$HOME/.cargo/env" && git clone https://github.com/pathcosmos/mig-gpu-mon.git /tmp/mig-gpu-mon && cargo install --path /tmp/mig-gpu-mon && mig-gpu-mon --help
```

### 다른 서버에 바이너리만 복사 (Rust 없이)

같은 아키텍처(x86_64 Linux)의 다른 서버에 빌드된 바이너리만 복사하면 된다.
대상 서버의 glibc 버전이 빌드 서버와 같거나 높아야 한다 (`ldd --version`으로 확인).

```bash
# 빌드 서버에서
scp target/release/mig-gpu-mon user@target-server:/usr/local/bin/

# 대상 서버에서 (Rust 설치 불필요)
mig-gpu-mon
```

### 제거

```bash
cargo uninstall mig-gpu-mon      # cargo install로 설치한 경우
# 또는
rm ~/.cargo/bin/mig-gpu-mon      # install.sh로 설치한 경우
rm /usr/local/bin/mig-gpu-mon    # 수동 복사한 경우
```

## Build & Install (상세)

```bash
# 릴리즈 빌드 (최적화 + LTO + strip)
cargo build --release

# 바이너리 위치
ls -lh target/release/mig-gpu-mon  # ~1.5MB

# 시스템 경로에 설치
cp target/release/mig-gpu-mon /usr/local/bin/

# 또는 직접 실행
./target/release/mig-gpu-mon
```

## Usage

```bash
# 기본 실행 (1초 간격)
mig-gpu-mon

# 폴링 간격 조정 (밀리초)
mig-gpu-mon --interval 2000    # 2초 간격 (리소스 절약)
mig-gpu-mon -i 500             # 0.5초 간격 (빠른 반응)

# NVML 라이브러리 경로 수동 지정 (자동 탐지 실패 시)
mig-gpu-mon --nvml-path /usr/local/nvidia/lib64/libnvidia-ml.so.1

# 도움말
mig-gpu-mon --help
```

### 키보드 조작

| 키 | 동작 |
|----|------|
| `Tab` / `↓` / `→` | 다음 GPU/MIG 인스턴스 |
| `Shift+Tab` / `↑` / `←` | 이전 GPU/MIG 인스턴스 |
| `q` / `Ctrl+C` | 종료 |

## Tech Stack

| 역할 | 크레이트 | 용도 |
|------|----------|------|
| GPU Metrics | `nvml-wrapper` + `nvml-wrapper-sys` | NVML C API 바인딩, MIG FFI 직접 호출 |
| TUI Rendering | `ratatui` + `crossterm` | sparkline, gauge, 레이아웃 위젯 |
| System Metrics | `sysinfo` | CPU 코어별 사용률, RAM/Swap |
| CLI | `clap` | 인자 파싱 |
| Error Handling | `anyhow` | 에러 체인 |

## Architecture

```
src/
  main.rs           진입점, 메인 루프 (collect → draw → event poll)
  app.rs            앱 상태 (메트릭, 히스토리, 선택 GPU)
  event.rs          키보드 / tick 이벤트 핸들링
  gpu/
    mod.rs          모듈 선언
    nvml.rs         NVML 래퍼 + MIG raw FFI + 디바이스 캐시
    metrics.rs      GPU/시스템 메트릭 구조체 + VecDeque 링 버퍼 히스토리
  ui/
    mod.rs          모듈 선언
    dashboard.rs    전체 TUI 레이아웃 및 위젯 렌더링
```

### 메인 루프 흐름

```
loop {
    tick_start = Instant::now()
    1. GPU 메트릭 수집 (NVML API)
       - 물리 GPU: utilization_rates(), memory_info(), temperature(), ...
       - MIG 인스턴스: utilization_rates() 실패 시
         → nvmlDeviceGetProcessUtilization() 폴백으로 SM/Mem util 집계
    2. 시스템 메트릭 수집 (sysinfo)
       - CPU 코어별 usage, 총 RAM/Swap
    3. TUI 렌더링 (ratatui)
    4. CPU 버퍼 재활용 (이전 SystemMetrics에서 회수 → zero-alloc)
    5. 이벤트 대기 (crossterm poll, interval - elapsed 만큼 블로킹)
       - 드리프트 보정: 작업 시간을 빼고 남은 시간만 poll
       - 키 입력 처리 또는 tick → 다음 루프
}
```

## MIG Utilization 수집 메커니즘

### 4단계 폴백 아키텍처

MIG 환경에서 GPU/Memory utilization 수집은 계단식 폴백 전략을 사용한다:

```
nvmlDeviceGetMigDeviceHandleByIndex(parent, idx)
    → mig_handle

1단계: nvmlDeviceGetUtilizationRates(mig_handle)
    → 성공: gpu_util, memory_util 직접 사용
    → 실패 (NVML_ERROR_NOT_SUPPORTED): 2단계로 진행

2단계: nvmlDeviceGetProcessUtilization(mig_handle, samples, &count, 0)
    → 1차 호출: count=0 → NVML_ERROR_INSUFFICIENT_SIZE, count에 필요 크기 반환
    → 2차 호출: 버퍼 전달 → 프로세스별 smUtil, memUtil 수집
    → max(smUtil), max(memUtil) 집계하여 인스턴스 레벨 값 산출
    → 모든 샘플이 0이거나 fetch 실패 시: 3단계로 진행

3단계: nvmlDeviceGetSamples(parent_handle, GPU_UTILIZATION_SAMPLES)
    → 부모 GPU에서 raw utilization 샘플 수집 (20ms 간격)
    → 최근 5개 샘플 평균, /10000으로 0-100% 스케일 변환
    → MIG 슬라이스 비율로 스케일링: mig_util = parent_util × total_slices / mig_slices
    → 예: parent=29%, MIG 3g.40gb → 29% × 7/3 ≈ 67%
    → 불가능 시: "N/A" 표시

4단계: nvmlGpmMigSampleGet() — memory_util 전용 (Hopper+ only)
    → DeviceInfo 캐시에서 GPM 지원 여부 확인
    → MIG: nvmlGpmMigSampleGet(parent_handle, gpuInstanceId, sample)
    → 일반 GPU: nvmlGpmSampleGet(device, sample)
    → 이전 tick 샘플과 현재 샘플로 nvmlGpmMetricsGet() 호출
    → NVML_GPM_METRIC_DRAM_BW_UTIL (ID 10) → 0-100%
    → Ampere 및 이전 아키텍처: GPM 미지원 → "N/A" 유지
```

### 드라이버 535.x MIG 제한사항 (심층 조사)

**H100 PCIe + MIG 3g.40gb + 드라이버 535.129.03** 환경에서 광범위한 테스트 결과, 이 드라이버 버전에서는 **어떤 표준 NVML API로도** MIG 인스턴스별 GPU utilization 또는 메모리 컨트롤러 utilization을 수집할 수 없음을 확인했다.

#### NVML API 테스트 결과 (드라이버 535.129.03)

| NVML API | 부모 GPU | MIG 인스턴스 |
|---|---|---|
| `nvmlDeviceGetUtilizationRates()` | NotSupported (ret=3) | NotSupported (ret=3) |
| `nvmlDeviceGetProcessUtilization()` | size query OK → fetch NotSupported | size query OK → fetch InvalidArg (ret=2) |
| `nvmlDeviceGetSamples(GPU_UTIL)` | **동작** (119개 샘플, raw 값) | InvalidArg (ret=2) |
| `nvmlDeviceGetSamples(MEM_UTIL)` | NotSupported | InvalidArg |
| `nvmlDeviceGetFieldValues(GPU_UTIL=203)` | FAIL (ret=2) | "OK"이지만 val=0 (더미 데이터) |
| `nvmlDeviceGetFieldValues(MEM_UTIL=204)` | FAIL (ret=2) | "OK"이지만 val=0 (더미 데이터) |

#### 디바이스별 메트릭 가용성

| 메트릭 | MIG 인스턴스 | 부모 GPU |
|---|---|---|
| VRAM 사용/총량 | OK | NoPermission |
| Temperature | InvalidArg | OK |
| Power usage | NotSupported | OK |
| Clock speeds | NotSupported | OK |
| PCIe throughput | InvalidArg | OK |
| Process list | OK | - |
| `nvmlDeviceGetSamples(GPU_UTIL)` | InvalidArg | **OK** (raw 값 ~290000 범위) |

#### Raw 샘플 값 해석

부모 디바이스의 `nvmlDeviceGetSamples(GPU_UTILIZATION_SAMPLES)` 반환값:
- ~119개 샘플, 20ms 간격
- 값 범위: ~230000-340000 (0-100%가 아님)
- 10000으로 나누면 합리적인 utilization 퍼센트 산출 (~29%)
- 여러 라운드에서 안정적으로 확인 (avg=291419, 292075, 292760)

#### MIG 슬라이스 비율 스케일링

MIG별 utilization 직접 수집이 불가하므로, 부모의 집계된 utilization을 비례 배분:
- MIG 디바이스 속성에서 `gpuInstanceSliceCount` 제공 (예: 3g.40gb → 3)
- `MaxMigDeviceCount`로 전체 슬라이스 수 확인 (예: H100 → 7)
- 공식: `mig_util = parent_util × total_slices / mig_slices`
- 예: parent=29%, slices=3/7 → MIG util ≈ 67%

이것은 **근사값**이다 — 부모의 모든 utilization이 해당 MIG 인스턴스에서 발생한다고 가정한다. 여러 MIG 인스턴스가 동시에 활성화되면 부모 utilization이 분배된다.

#### 메모리 컨트롤러 Utilization — 전수 조사 결과

MIG 환경에서 메모리 컨트롤러(Memory Controller) utilization을 수집하기 위해 가능한 **모든 NVML API**를 조사했다.

##### 시도한 API 목록 및 결과

| # | API / 접근법 | 부모 GPU | MIG 인스턴스 | 판정 |
|---|---|---|---|---|
| 1 | `nvmlDeviceGetUtilizationRates().memory` | NotSupported | NotSupported | ❌ MIG 공식 미지원 |
| 2 | `nvmlDeviceGetProcessUtilization()` → `memUtil` | fetch 실패 | InvalidArg | ❌ 0 또는 에러 |
| 3 | `nvmlDeviceGetSamples(MEM_UTIL)` | **NotSupported** | InvalidArg | ❌ 부모에서도 불가 → 스케일링 원천 차단 |
| 4 | `nvmlDeviceGetFieldValues(MEM_UTIL=204)` | FAIL (ret=2) | "OK" but val=0 | ❌ 더미 데이터 |
| 5 | `nvidia-smi dmon` mem% | — | MIG 미지원 | ❌ nvidia-smi 자체 제한 |
| 6 | CUDA `cudaMemGetInfo` | 용량만 | 용량만 | ❌ 컨트롤러 활용률 아닌 용량 |
| 7 | `nvmlDeviceGetMemoryBusWidth` | 정적값 | 정적값 | ❌ 버스 폭(bit)이지 utilization 아님 |
| 8 | 드라이버 545/550/555 | — | — | ❌ 표준 API 제한 해제 없음 |
| 9 | **NVML GPM `DRAM_BW_UTIL`** | — | **Hopper+ 동작** | ✅ 유일한 경로 |

##### GPU Util과의 핵심 차이

GPU Util은 부모 GPU의 `nvmlDeviceGetSamples(GPU_UTIL)`이 **동작**하여 MIG 슬라이스 비율 스케일링이 가능했다. 그러나 `MEM_UTIL`은 **부모 GPU에서조차 NotSupported**이므로 스케일링할 원본 데이터 자체가 존재하지 않는다.

##### GPM (GPU Performance Monitoring) — Hopper+ 전용 솔루션

NVML GPM API는 드라이버 520+에서 **Hopper (H100) 이상** 아키텍처에 도입되었다. `NVML_GPM_METRIC_DRAM_BW_UTIL` (ID 10)은 이론적 최대 대비 DRAM 대역폭 사용률(0.0~100.0%)을 제공하며, MIG 인스턴스에서도 `nvmlGpmMigSampleGet()`으로 수집 가능하다.

```
GPM 수집 흐름:
1. nvmlGpmQueryDeviceSupport(parent_device) → GPM 지원 확인 (DeviceInfo 캐시)
   ⚠ 반드시 부모 GPU 핸들로 체크 — MIG 핸들에서는 에러 반환 + NVML 상태 오염
2. nvmlGpmSampleAlloc() → 샘플 버퍼 할당
3. nvmlGpmMigSampleGet(parent, gpuInstanceId, sample) — MIG 인스턴스
   nvmlGpmSampleGet(device, sample) — 일반 GPU (non-MIG only)
   ⚠ MIG 핸들에 nvmlGpmSampleGet 호출 금지 — NVML 상태 오염 → VRAM 등 후속 쿼리 실패
4. 이전 tick의 샘플과 현재 샘플로 nvmlGpmMetricsGet() 호출
5. metrics[0].value → DRAM BW Util (0.0~100.0%)

collect_mig_instances 2-phase 수집 (v0.3 적용):
  Phase 1: 모든 MIG 인스턴스의 VRAM + utilization 수집 (GPM 호출 없음)
           → memory_info(), utilization_rates(), process util, sample scaling
  Phase 2: GPM fallback으로 memory_util(Mem Ctrl) 수집 (Hopper+ only)
           → nvmlGpmMigSampleGet(parent, gi_id, sample)
  목적: GPM 호출이 NVML 상태를 오염시켜도 Phase 1에서 VRAM이 이미 수집 완료
```

| GPU 아키텍처 | GPM 지원 | Mem Ctrl 표시 |
|---|---|---|
| Ampere (A100/A30) | ❌ | "N/A" 유지 |
| Hopper (H100/GH200) | ✅ | DRAM BW Util % |
| Blackwell+ | ✅ | DRAM BW Util % |

> **구현 상태:** 본 도구는 GPM DRAM BW Util 수집을 이미 구현하였으며, Hopper+ GPU에서 자동 활성화된다. 첫 tick은 baseline 수집(None), 2번째 tick부터 실측값 표시.

> **참고:** NVIDIA 드라이버 550+ (CUDA 12.4+)부터 MIG 디바이스 핸들에서 `nvmlDeviceGetUtilizationRates()` 정식 지원이 추가되어, 3단계 폴백이 불필요해진다.

#### VRAM + Mem Ctrl 동시 표시 버그 분석 및 수정 (v0.3)

##### 증상

MIG 환경에서 VRAM이 첫 tick에만 표시되고 이후 `0/0 MB`로 사라지며, Mem Ctrl만 "N/A" 또는 값으로 표시되는 현상. 둘 다 동시에 정상 표시되어야 함.

##### 근본 원인 분석 — 3가지 연쇄 버그

**버그 1: `get_device_info`의 GPM 쿼리가 MIG 핸들에서 NVML 상태 오염**

```
collect_device_metrics() 호출 순서 (수정 전):
  line 543: get_device_info(mig_device)
            → nvmlGpmQueryDeviceSupport(mig_handle)  ← NVML 상태 오염!
  line 546: memory_info()                             ← 오염된 상태에서 VRAM 쿼리 → 실패
```

`get_device_info()`가 MIG 핸들에 대해 `nvmlGpmQueryDeviceSupport()`를 호출. 이 GPM 쿼리가 NVML 드라이버 내부 상태를 오염시켜, 바로 아래의 `memory_info()` VRAM 쿼리가 실패하거나 `(0, 0)` 반환. DeviceInfo 캐시 덕분에 첫 tick에서만 발생하지만, 버그 2와 결합하면 매 tick 문제.

**버그 2: 크로스-틱 GPM 상태 오염 (핵심 메커니즘)**

```
Tick N:  VRAM 쿼리(성공) → GPM fallback(nvmlGpmMigSampleGet) → NVML 상태 오염
Tick N+1: VRAM 쿼리(실패 — 이전 tick GPM 오염 잔존) → GPM fallback → 또 오염
Tick N+2: VRAM 쿼리(실패) → ...
```

`collect_mig_instances`에서 GPM fallback(`nvmlGpmMigSampleGet`)이 VRAM 쿼리 이후에 실행되지만, 이 GPM 호출이 NVML 드라이버 상태를 오염시키면 **다음 tick**의 VRAM 쿼리가 실패. 같은 함수 내에서 순서를 바꾸는 것만으로는 해결 불가 — tick 간 상태가 지속됨.

**버그 3: `memory_used`/`memory_total`이 실패를 은닉**

```rust
// 수정 전: unwrap_or((0, 0)) — 실패 시 0/0으로 조용히 사라짐
let (memory_used, memory_total) = device.memory_info()
    .map(|m| (m.used, m.total))
    .unwrap_or((0, 0));  // ← VRAM 0/0 MB (0.0%) — 사용자는 "비활성화"로 인식
```

VRAM 쿼리 실패 시 `u64` 타입이라 `(0, 0)`으로 fallback되어 "VRAM 0/0 MB (0.0%)"로 표시. `memory_util`은 `Option<u32>`라서 "Mem Ctrl N/A"로 명시적 표시되는 것과 대조적. 사용자 입장에서 VRAM이 "사라진" 것처럼 보임.

##### 타임라인 재현

```
Tick 1 (첫 tick):
  ├── get_device_info(mig) → nvmlGpmQueryDeviceSupport(mig_handle) [첫 호출, 캐시 미스]
  │   → NVML 상태 오염 가능 (but 캐시되어 이후 호출 없음)
  ├── memory_info() → 성공 또는 실패 (오염 정도에 따라)
  ├── utilization_rates() → NVML_ERROR_NOT_SUPPORTED (MIG 제한)
  ├── process_util fallback → sm/mem util 수집
  └── GPM fallback → nvmlGpmMigSampleGet(parent, gi_id) → 첫 tick이라 prev_sample 없음 → None
      → 그러나 GPM 호출 자체가 NVML 상태 오염

Tick 2 (이후 tick):
  ├── get_device_info(mig) → 캐시 히트 (GPM 쿼리 안 함)
  ├── memory_info() → 실패! (Tick 1 GPM 호출의 잔존 오염)
  │   → unwrap_or((0, 0)) → VRAM 0/0 MB ← 사용자가 보는 "비활성화"
  ├── ... (나머지 동일)
  └── GPM fallback → nvmlGpmMigSampleGet → prev_sample 있음 → memory_util 값 반환!
      → 그러나 또 NVML 상태 오염 → Tick 3 VRAM도 실패

결과: VRAM은 Tick 1에서만 표시, Tick 2+에서 0/0 MB
      Mem Ctrl은 Tick 2+에서 값 표시 (또는 Ampere에서 항상 N/A)
```

##### 수정 내용 (3건)

**수정 1: `get_device_info`에서 MIG 핸들 GPM 쿼리 차단** (`nvml.rs`)

```rust
// 수정 전: 모든 디바이스에 대해 GPM 쿼리 실행
fn get_device_info(&self, device: &Device) -> DeviceInfo {
    gpm_supported: nvmlGpmQueryDeviceSupport(device.handle(), ...)  // MIG 핸들 → 오염!
}

// 수정 후: skip_gpm_query 파라미터 추가
fn get_device_info(&self, device: &Device, skip_gpm_query: bool) -> DeviceInfo {
    gpm_supported: if skip_gpm_query { false } else { nvmlGpmQueryDeviceSupport(...) }
}
// MIG 핸들: get_device_info(mig_device, true)  → GPM 쿼리 스킵
// Parent:   get_device_info(parent_device, false) → GPM 쿼리 정상 실행
```

**수정 2: `collect_mig_instances` 2-phase 분리** (`nvml.rs`)

```rust
// 수정 전: MIG 인스턴스별 VRAM + GPM 인터리브
for mig in mig_instances {
    metrics = collect_device_metrics(mig)  // VRAM 쿼리
    gpm_fallback(mig)                      // GPM 호출 → 다음 MIG의 VRAM 쿼리 오염
}

// 수정 후: 2-phase 분리
// Phase 1: 모든 MIG VRAM 수집 (GPM 호출 없음)
for mig in mig_instances {
    metrics = collect_device_metrics(mig)  // VRAM + utilization + process util
    phase1.push(metrics)
}
// Phase 2: GPM fallback (모든 VRAM 이미 수집 완료)
for metrics in &mut phase1 {
    gpm_fallback(metrics)  // GPM 호출 → 오염되어도 VRAM에 영향 없음
}
```

**수정 3: `memory_used`/`memory_total` → `Option<u64>`** (`metrics.rs` + `dashboard.rs`)

```rust
// 수정 전: u64 — 실패 시 (0, 0)으로 은닉
pub memory_used: u64,
pub memory_total: u64,
// → "VRAM 0/0 MB (0.0%)" — 사용자에게 혼란

// 수정 후: Option<u64> — 실패 시 "N/A" 명시
pub memory_used: Option<u64>,
pub memory_total: Option<u64>,
// → "VRAM N/A" — gpu_util, memory_util과 동일한 패턴
```

UI도 동시 업데이트:
- Detail 패널: `VRAM N/A` 표시 (DarkGray 색상)
- Sparkline 타이틀: `VRAM N/A` 표시
- 히스토리: `Some`일 때만 push → 실패 tick에서 그래프 데이터 오염 방지

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/gpu/nvml.rs` | `get_device_info(skip_gpm_query)` 파라미터 추가, `collect_mig_instances` 2-phase 분리, MIG 호출자에서 `skip_gpm_query=true` 전달 |
| `src/gpu/metrics.rs` | `memory_used`/`memory_total` → `Option<u64>`, `memory_used_mb()`/`memory_total_mb()`/`memory_percent()` → `Option` 반환 |
| `src/ui/dashboard.rs` | VRAM detail/sparkline에 `N/A` fallback 추가, `vram_max` 계산에 `and_then` 사용 |

##### 상호 검증 매트릭스

| 시나리오 | VRAM 표시 | Mem Ctrl 표시 | 검증 |
|----------|-----------|-------------|------|
| Hopper+ MIG, Tick 1 | 정상값 (Phase 1에서 수집) | N/A (GPM 첫 tick, prev_sample 없음) | Phase 1에서 VRAM 수집 → Phase 2 GPM 오염 무관 |
| Hopper+ MIG, Tick 2+ | 정상값 (Phase 1) | 정상값 (Phase 2 GPM delta) | GPM 오염이 있어도 VRAM은 Phase 1에서 이미 완료 |
| Ampere MIG | 정상값 | N/A (GPM 미지원) | GPM 호출 자체가 없음 → VRAM 오염 불가 |
| Non-MIG GPU | 정상값 | 정상값 또는 GPM값 | GPM은 non-MIG에서만 직접 호출, VRAM 먼저 수집 |
| memory_info() 실패 | "VRAM N/A" | 별도 경로 | Option<u64>로 명시적 실패 표시 |
| get_device_info 첫 호출 (MIG) | 정상값 | — | skip_gpm_query=true → GPM 쿼리 스킵 → NVML 오염 없음 |

#### VRAM 정체(Stagnation) 버그 분석 및 수정 (v0.3.1)

##### 증상

MIG 환경에서 VRAM이 초반 몇 tick에 정상 표시되다가 이후 값이 **정체**(고정)된다. 텍스트 값이 변하지 않고, sparkline 그래프도 멈춤.

##### 근본 원인 분석 — 2가지 연쇄 버그

**버그 1: 크로스-틱 GPM 오염이 `memory_info()` 실패를 유발**

v0.3에서 2-phase 분리로 **같은 tick 내** GPM→VRAM 오염을 차단했지만, GPM 호출이 남긴 NVML 드라이버 상태 오염은 **다음 tick까지 지속**된다.

```
Tick N:   Phase 1(VRAM 성공) → Phase 2(GPM 호출 → NVML 상태 오염)
Tick N+1: Phase 1(memory_info() 실패 — 이전 tick GPM 오염 잔존) → memory_used = None
Tick N+2: Phase 1(또 실패) → ...
```

2-phase는 같은 tick 내 보호만 제공. **tick 간** 오염 잔존은 별도 대응 필요.

**버그 2: `MetricsHistory::push()`가 `None`일 때 push 안 함 → sparkline 정체**

```rust
// 수정 전: None이면 push 자체를 건너뜀
if let Some(val) = metrics.memory_used_mb() {
    Self::push_ring(&mut self.memory_used_mb, val, self.max_entries);
}
// → memory_info() 실패 시 ring buffer 업데이트 중단 → sparkline 고정
```

`memory_used`가 `None`이면 `memory_used_mb` 링 버퍼에 아무것도 push되지 않아 sparkline이 마지막 성공 값에서 멈춤. 동일 문제가 `gpu_util`, `memory_util`, `temperature` 등 모든 sparkline 메트릭에 존재.

##### 수정 내용 (2건)

**수정 1: `update_metrics()`에서 VRAM carry-forward** (`app.rs`)

```rust
// 수정 전: 그대로 저장
pub fn update_metrics(&mut self, new_metrics: Vec<GpuMetrics>) { ... }

// 수정 후: memory_used가 None이면 이전 tick의 동일 UUID에서 마지막 값 계승
pub fn update_metrics(&mut self, mut new_metrics: Vec<GpuMetrics>) {
    for m in &mut new_metrics {
        if m.memory_used.is_none() {
            if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) {
                m.memory_used = prev.memory_used;
                m.memory_total = prev.memory_total;
            }
        }
    }
    // ... 기존 로직
}
```

- GPM 오염으로 `memory_info()`가 실패해도 텍스트 표시가 "N/A"로 빠지지 않음
- UUID 기반 매칭으로 MIG 인스턴스 간 혼선 없음

**수정 2: `MetricsHistory::push()`에 `push_or_repeat()` 적용** (`metrics.rs`)

```rust
// 수정 전: None이면 push 건너뜀
if let Some(val) = metrics.gpu_util {
    Self::push_ring(&mut self.gpu_util, val, self.max_entries);
}

// 수정 후: None이면 마지막 값 반복 push → sparkline 계속 롤링
fn push_or_repeat<T: Copy>(buf: &mut VecDeque<T>, val: Option<T>, max: usize) {
    let v = match val {
        Some(v) => v,
        None => match buf.back() {
            Some(&last) => last,
            None => return,  // 한 번도 관측 안 된 메트릭은 데이터 조작 방지
        },
    };
    Self::push_ring(buf, v, max);
}
```

모든 sparkline 메트릭(`gpu_util`, `memory_util`, `memory_used_mb`, `sm_util`, `temperature`, `power_usage_w`, `clock_graphics_mhz`, `pcie_tx_kbps`, `pcie_rx_kbps`)에 동일 적용.

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/app.rs` | `update_metrics()` — VRAM carry-forward (이전 tick 동일 UUID에서 계승) |
| `src/gpu/metrics.rs` | `push_or_repeat()` — 모든 sparkline 메트릭에 None 시 마지막 값 반복 push |

##### 상호 검증 매트릭스

| 시나리오 | VRAM 텍스트 | VRAM sparkline | 검증 |
|----------|------------|---------------|------|
| Tick 1 (정상) | 정상값 | 정상 push | Phase 1에서 수집 성공 |
| Tick 2+ (GPM 오염으로 memory_info 실패) | 이전 값 유지 (carry-forward) | 이전 값 반복 push (rolling) | update_metrics에서 계승 + push_or_repeat |
| Tick 2+ (memory_info 정상 복구) | 새 값 표시 | 새 값 push | carry-forward는 None일 때만 동작 |
| GPU util 일시 None | 마지막 값 유지 | 마지막 값 반복 push | push_or_repeat 적용 |
| 한 번도 관측 안 된 메트릭 | N/A | push 안 함 | `buf.back() == None` → return, 데이터 조작 방지 |
| Ampere MIG (GPM 없음) | 정상값 | 정상 push | GPM 호출 없어 오염 불가 |
| Non-MIG GPU | 정상값 | 정상 push | memory_info() 정상 동작 |

##### v0.3 수정과의 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3 Fix 1: `skip_gpm_query` | MIG 핸들에서 GPM 쿼리 차단 → 첫 호출 오염 방지 | `get_device_info()` |
| v0.3 Fix 2: 2-phase 분리 | 같은 tick 내 GPM→VRAM 오염 차단 | `collect_mig_instances()` |
| v0.3 Fix 3: `Option<u64>` | VRAM 실패 시 0/0 대신 "N/A" 명시 | `metrics.rs` |
| **v0.3.1 Fix 1: carry-forward** | **tick 간 GPM 오염 잔존 시 VRAM 값 계승** | `app.rs:update_metrics()` |
| **v0.3.1 Fix 2: push_or_repeat** | **None 발생 시 sparkline 정체 방지** | `metrics.rs:push()` |

v0.3은 "오염 자체를 차단"하는 방어, v0.3.1은 "오염이 발생해도 표시를 유지"하는 복원력(resilience) 계층.

### NVML API 지연시간 벤치마크

H100 PCIe에서 API 호출당 1000회 반복 측정:

| API 호출 | 평균 지연 | 비고 |
|---|---|---|
| `nvmlDeviceGetSamples(GPU_UTIL)` 2-phase | **835 µs** | MIG 폴백 신규 추가 |
| `nvmlDeviceGetUtilizationRates()` (실패) | 202 µs | 실패 경로도 빠름 |
| `temperature()` | 234 µs | 단순 센서 읽기 |
| `power_usage()` | 3,592 µs | 가장 비싼 호출 (하드웨어 SMBus) |
| `clock_info(Graphics)` | 1,489 µs | 보통 |
| `memory_info()` | 7 µs | 가장 빠름 |

신규 `nvmlDeviceGetSamples` 호출은 tick당 ~835µs 추가 — 1초 인터벌에서 0.1% 미만 오버헤드이며, 기존 `power_usage()` 호출보다 가볍다.

버퍼: 128 entries × 16 bytes = 2,048 bytes (`RefCell<Vec>` 재사용, tick당 할당 없음).

## Performance Optimization

모니터링 도구 자체가 GPU 워크로드에 영향을 주지 않도록 리소스 사용을 극소화했다.

### 예상 리소스 소모량

기본 설정(1초 간격) 기준, GPU 1대 + MIG 2인스턴스 환경에서의 예상치:

| 리소스 | 예상 소모량 | 비고 |
|--------|-------------|------|
| **CPU** | **0.5~2.5% (1코어 기준)** | tick당 활성 시간 ~7-20ms, 나머지 ~980-993ms sleep |
| **RSS 메모리** | **4~8 MB** | 바이너리 + libnvidia-ml.so + 히스토리 버퍼 + TUI 버퍼 |
| **GPU 연산 자원** | **0% (사용 안 함)** | NVML은 읽기 전용 드라이버 IPC, CUDA 컨텍스트 미생성 |
| **GPU VRAM** | **0 MB (사용 안 함)** | GPU 메모리 할당 없음 |
| **디스크 I/O** | **사실상 0** | `/proc` (procfs 가상 FS) 읽기만 — 실제 디스크 접근 없음 |
| **네트워크** | **0** | 네트워크 통신 없음 |

#### tick당 시간 분해 (1초 interval 기준)

```
1 tick = 1000ms
├── NVML API 호출         ~7-18ms   드라이버 IPC (GPU당 15-19개 쿼리)
│   ├── device_by_index        ~0.1ms
│   ├── utilization_rates      ~0.5ms
│   ├── memory_info            ~0.5ms
│   ├── temperature            ~0.3ms
│   ├── power_usage            ~0.3ms
│   ├── power_management_limit ~0.3ms
│   ├── clock_info (×3)        ~0.5ms   Graphics/SM/Memory
│   ├── pcie_throughput (×2)   ~0.3ms   TX/RX
│   ├── pcie_link_gen/width    ~0.1ms
│   ├── performance_state      ~0.1ms
│   ├── throttle_reasons       ~0.1ms
│   ├── encoder/decoder_util   ~0.2ms
│   ├── ecc_errors (×2)        ~0.2ms   Corrected/Uncorrected
│   ├── running_compute_procs  ~0.5ms   + top-5만 /proc 이름 읽기 (지연 I/O)
│   └── (MIG) process_util     ~1-3ms   MIG 인스턴스당, 폴백 시에만
│   └── (MIG) gpu_util_samples ~0.8ms   nvmlDeviceGetSamples 폴백, 부모 GPU만
├── sysinfo refresh       ~0.1-0.3ms
│   ├── refresh_cpu_usage      ~0.1ms   /proc/stat 읽기
│   └── refresh_memory         ~0.05ms  /proc/meminfo 읽기
├── TUI 렌더링            ~0.5-2ms   ratatui diff buffer + ANSI 출력
├── 이벤트 대기 (sleep)   ~980-993ms  crossterm poll, 커널 스케줄링
└── 합계 활성 시간        ~7-20ms    = CPU 0.7-2.0%
```

#### RSS 메모리 분해

```
총 RSS ~4-8 MB
├── 바이너리 코드/데이터 세그먼트        ~1.4 MB   (mmap)
├── libnvidia-ml.so 공유 라이브러리      ~2-4 MB   (mmap, 시스템 공유)
├── 히스토리 링 버퍼                     ~80 KB
│   ├── GPU당 MetricsHistory              ~22 KB   (9 VecDeque × 300 × 4-8B)
│   │   (× 3 디바이스 = ~42 KB)
│   └── SystemHistory                     ~5 KB    (2 VecDeque × 300 × 4-8B)
├── ratatui Terminal 더블 버퍼           ~50-400 KB (터미널 크기에 비례)
│   (80×24: ~77KB, 200×50: ~400KB)
├── sysinfo System 구조체               ~30-50 KB  (CPU만, 프로세스 제외)
├── 재사용 버퍼들                        ~5 KB
│   ├── thread_local sparkline buf        ~2.4 KB
│   ├── proc_sample_buf                   ~1 KB
│   ├── gpu_sample_buf                    ~2 KB
│   └── cpu_buf                           ~0.3 KB
└── HashMap, String 캐시 등              ~5-10 KB
```

#### interval 별 리소스 비교

| interval | CPU 사용률 | 특성 |
|----------|-----------|------|
| `500ms` | ~1.5-4% | 빠른 반응, 모니터링 부하 약간 증가 |
| `1000ms` (기본) | ~0.7-2.0% | 균형잡힌 기본값 |
| `2000ms` | ~0.4-1.0% | 리소스 절약, 대규모 클러스터용 |
| `5000ms` | ~0.1-0.4% | 최소 부하, 장기 관찰용 |

> 메모리(RSS)는 interval에 관계없이 동일. 히스토리 엔트리 수(300)가 고정이므로
> interval이 길수록 더 긴 시간 범위를 기록한다 (1초: 5분, 2초: 10분, 5초: 25분).

### 최적화 상세: Memory

| 최적화 | 위치 | Before → After |
|--------|------|----------------|
| `VecDeque` 링 버퍼 | `metrics.rs` | `Vec::remove(0)` O(n) memmove → `VecDeque::pop_front()` O(1) |
| 디바이스 정보 캐시 | `nvml.rs` | 매 tick NVML API + String 할당 → `RefCell<HashMap>` 첫 호출만, 이후 캐시 히트 |
| process sample 버퍼 | `nvml.rs` | MIG 호출마다 `vec![zeroed(); N]` 할당/해제 → `RefCell<Vec>` grow-only 재사용 |
| CPU 버퍼 zero-copy 스왑 | `main.rs` | 매 tick `Vec::clone()` → `std::mem::take` + 이전 SystemMetrics에서 버퍼 회수 (첫 tick 이후 할당 0) |
| sparkline 변환 버퍼 | `dashboard.rs` | draw마다 5개 `Vec<u64>` 할당 → `thread_local!` scratch 1개 재사용 |
| 프로세스 partial sort | `nvml.rs` | O(n log n) 전체 정렬 → O(n) `select_nth_unstable_by` (프로세스 > 5일 때) |
| 프로세스 이름 지연 읽기 | `nvml.rs` | 모든 프로세스 N개의 `/proc/{pid}/comm` 읽기 → top-5 선별 후 5개만 읽기 (I/O 최대 95% 감소) |
| CPU cores Vec 재사용 | `dashboard.rs` | 매 draw마다 Vec 할당 → `thread_local!` 버퍼 재사용 |
| `make_bar()` 문자열 | `dashboard.rs` | `.repeat()` 2회 연결 → `String::with_capacity` + push loop |
| HashMap uuid clone | `app.rs` | 매 tick `uuid.clone()` → `contains_key` 후 miss 시에만 clone |
| GPU 히스토리 자동 정리 | `app.rs` | MIG 재구성/GPU 제거 시 HashMap 엔트리 무한 증가 → `retain()`으로 사라진 UUID 자동 제거 |
| GPM 샘플·디바이스 캐시 자동 정리 | `nvml.rs` | MIG 재구성 시 stale handle의 `nvmlGpmSample_t` + `DeviceInfo` 무한 잔류 → 매 tick active handle 추적 후 `retain()` + `nvmlGpmSampleFree()` |
| NVML 샘플 버퍼 shrink | `nvml.rs` | grow-only 버퍼 무한 증가 가능 → capacity > needed×4 시 `shrink_to(needed×2)` 자동 축소 |
| `format_pstate` 제로 할당 | `nvml.rs` | 매 tick `"P0".to_string()` String 할당 → `&'static str` 반환 (할당 0) |
| `format_architecture` 제로 할당 | `nvml.rs` | 동일 패턴: `"Ampere".to_string()` → `&'static str` |
| `format_throttle_reasons` Vec 제거 | `nvml.rs` | `Vec::new()` + `push` + `join()` → macro로 `String`에 직접 append (Vec 할당 제거) |

### 최적화 상세: CPU (시스템 호출 최소화)

| 최적화 | 위치 | 효과 |
|--------|------|------|
| `System::new()` | `main.rs` | `new_all()` 대비 프로세스/디스크/네트워크 전체 스캔 제거 |
| 타겟 refresh | `main.rs` | `refresh_cpu_usage()` + `refresh_memory()`만 — /proc/stat, /proc/meminfo 2개만 읽기 |
| 기본 interval 1000ms | `main.rs` | 500ms 대비 모든 시스콜+NVML 호출 횟수 절반 |
| CPU priming | `main.rs` | sysinfo 첫 `refresh_cpu_usage()` 0% 반환 방지 — 초기화 시 1회 선호출 |
| 드리프트 보정 tick 루프 | `main.rs` | `작업시간 + interval` 누적 밀림 → `Instant` 기반 경과시간 측정, `interval - elapsed`만 poll |

### 최적화 상세: GPU (NVML 호출 최소화)

| 최적화 | 위치 | 효과 |
|--------|------|------|
| `utilization_rates()` 우선 | `nvml.rs` | MIG에서도 일단 시도, 실패 시에만 process util 폴백 (추가 IPC 2회 절약) |
| `nvmlDeviceGetSamples` 폴백 | `nvml.rs` | `utilization_rates()` MIG 실패 시 부모 GPU 레벨 샘플링 → 슬라이스 비율 스케일링, 버퍼 `RefCell<Vec>` 재사용 |
| process util 2-pass | `nvml.rs` | 1차 count=0으로 크기 확인, 2차 데이터 수집 — 과다 버퍼 할당 방지 |
| `RefCell` 내부 가변성 | `nvml.rs` | `&self`로 NVML 핸들 빌린 상태에서 캐시/버퍼 수정 가능, borrow checker 충돌 없이 |
| 프로세스 이름 지연 읽기 (top-5) | `nvml.rs` | 모든 프로세스 `/proc` 읽기 → pid+VRAM만 수집 후 top-5 선별 → 5개만 `/proc/{pid}/comm` 읽기 |
| GPM·디바이스 캐시 tick별 정리 | `nvml.rs` | active handle 추적 → stale `nvmlGpmSample_t` free + `DeviceInfo` 제거, MIG 재구성 시 NVML 리소스 누수 방지 |
| GPU 자원 0 사용 | 설계 | NVML은 읽기 전용 드라이버 쿼리 — CUDA 컨텍스트 없음, VRAM 할당 없음 |

### 최적화 상세: Binary Size

| 설정 | 값 | 효과 |
|------|-----|------|
| `opt-level` | 3 | 최대 최적화 |
| `lto` | true | Link-Time Optimization, 미사용 코드 제거 |
| `strip` | true | 디버그 심볼 완전 제거 |
| `codegen-units` | 1 | 단일 코드 생성 단위로 전체 최적화 (빌드 느려짐, 런타임 빨라짐) |
| `panic` | "abort" | unwind 코드 제거 — 바이너리 크기 감소 + 패닉 시 즉시 종료 |
| `tokio` 제거 | — | async 미사용, 동기 이벤트 루프로 충분 — 바이너리 ~200KB 절약 |
| 최종 크기 | **~1.5MB** | 단일 바이너리 (libc 동적 링크) |

## Runtime Stability (장시간 운영 안전성)

24/7 운영 시 메모리 증가나 리소스 누수 없이 안정적으로 동작하도록 설계되었다.

### 메모리 안정성

| 보호 메커니즘 | 위치 | 설명 |
|-------------|------|------|
| VecDeque 링 버퍼 (300 고정) | `metrics.rs` | GPU/시스템 히스토리 크기 고정, 무한 증가 불가 |
| GPU 히스토리 자동 정리 | `app.rs` | MIG 재구성/GPU 제거 시 orphan 엔트리 자동 삭제 |
| GPM 샘플·디바이스 캐시 정리 | `nvml.rs` | 매 tick active handle 추적 → stale `nvmlGpmSample_t` free + `DeviceInfo` 제거, MIG 재구성 반복 시에도 누수 없음 |
| NVML 샘플 버퍼 shrink-to-fit | `nvml.rs` | capacity > needed×4 시 자동 축소, 일시적 급증 후 복구 |
| DeviceInfo 캐시 1회 수집 | `nvml.rs` | 정적 정보(arch, CC 등)는 첫 호출 시 캐시, 이후 0 할당 |
| sysinfo targeted refresh | `main.rs` | `refresh_cpu_usage()` + `refresh_memory()`만 호출, 프로세스 누적 없음 |

### 장시간 메모리 사용량 예측

```
시작 직후:  ~4 MB RSS
5분 후:     ~5-8 MB RSS (히스토리 버퍼 300개 채워짐)
5분 이후:   변동 없음 (링 버퍼 → 정상 상태 유지)
```

### 런타임 안전성

| 보호 메커니즘 | 위치 | 설명 |
|-------------|------|------|
| 패닉 복구 훅 | `main.rs` | 패닉 발생 시 `disable_raw_mode()` + `LeaveAlternateScreen` 자동 호출 → 터미널 상태 복구 |
| 드리프트 보정 타이머 | `main.rs` | `Instant` 기반 경과시간 측정 → 작업 시간 빼고 남은 시간만 poll, 누적 밀림 방지 |
| Option 기반 graceful 실패 | `nvml.rs` | 모든 확장 메트릭 `.ok()` 래핑 → MIG/vGPU 실패 시 `None` ("N/A") 표시, 패닉 없음 |
| `saturating_sub` 시간 계산 | `main.rs` | 작업 시간 > interval 시에도 음수 없이 즉시 다음 틱 진행 |

## Why Rust

- **NVML FFI 직접 호출** — MIG 제한 우회를 위한 raw C API 접근 가능
- **제로 오버헤드** — 모니터링 도구 자체의 CPU/메모리 사용 극소화, GPU 워크로드에 영향 없음
- **단일 바이너리** — 클라우드/컨테이너 환경에서 `scp` 또는 `COPY`만으로 배포

## License

MIT
