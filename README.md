# mig-gpu-mon

**한국어** | [English](README_EN.md)

NVIDIA MIG(Multi-Instance GPU) 환경에서 `nvidia-smi`가 제공하지 못하는 GPU 메트릭을 실시간 모니터링하는 터미널 TUI 프로그램.

btop/nvtop 스타일의 실시간 sparkline 그래프를 터미널에서 표시하며, CPU 코어별 사용률과 시스템 RAM도 함께 모니터링한다.

> **Ubuntu 특화:** 개발 및 테스트가 Ubuntu 환경에서 이루어졌으며, 라이브러리 탐색 경로·에러 메시지·문서 모두 Ubuntu를 1차 대상으로 작성되었습니다. RHEL 계열, 컨테이너, WSL2 등 다른 환경에서도 동작하지만, Ubuntu에서 가장 매끄럽게 동작합니다.

## Screen Layout

### ASCII 구성도

```
┌─ mig-gpu-mon ────────────────────────────────── 2026-03-17 02:15:30 PM ┐
│                                        pathcosmos@gmail.com           │ ← Header
├─ CPU (64 cores) 23.4% ─────────┬─ Devices ────────────────────────────┤
│ 17 ▮▮▮▮▮▮▮  92%   5 ▮▮▮▯▯ 34% │ > MIG 0 (GPU 0: A100) GPU:45% Mem:… │ ↑ 20%
│  2 ▮▮▮▮▮▯▯  65%  40 ▮▮▯▯▯ 18% │   MIG 1 (GPU 0: A100) GPU:12% Mem:… │ ↓
│  0 ▮▮▮▮▯▯▯  52%  33 ▮▯▯▯▯  5% ├─ Detail ─────────────────────────────┤    ← Top 45%
│ 12 ▮▮▮▯▯▯▯  38%   8 ▯▯▯▯▯  2% │ Name: MIG 0 (GPU 0: A100-SXM4-80GB) │ ↑
│  ...                            │ UUID: MIG-a1b2...  Arch:Ampere CC:8.0│ │
│                                 │ VRAM 12288 MB / 20480 MB (60.0%)    │ │
│                                 │ GPU: 45%  Mem: 38%  SM: 45%         │ │ 50%
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
│ ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅            │ ▂▂▃▃▂▂▃▂▃▃▂▂▃▃▂▃                   │ ← 40%
├─ Mem Ctrl 38% ──────────────────┤RAM ▮▮▮▮▮▮▯▯▯▯▯▯▯▯▯▯▯▯▯             │ ← RAM/SWP 바 (텍스트 없음)
│ ▃▃▃▄▄▅▅▅▄▃▃▃▄▄▅▅▄             │SWP ▮▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯             │    ← Bottom 55%
├─ VRAM 12288/20480 MB (60.0%) ──│ ▮ used  ▮ cached  ▮ free  RAM …     │ ← Memory 범례 (2줄)
│ ▅▅▅▅▆▆▆▆▆▆▇▇▇▇▇▇▇             │ 70.1G/12.5G/6.6G  avl:77.5G        │
├─ PCIe TX:12.3 RX:56.7 MB/s ────├─ RAM ─────────────────────────────┤
│ ▂▃▃▅▅▆▅▃▂▂▃▅▆▆▅▃              │ ▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅ ← used+cached 색상│ (PCIe 있을 때)
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
│   │   │   └── CPU Cores         (전체 영역)  " CPU ({N} cores) {pct}% "
│   │   │       └── dynamic N-column bars   "{idx} ▮▮▯▯ {pct}%" (사용량 높은 순 정렬)
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
│           ├── CPU Total {pct}%       sparkline   40%
│           ├── RAM/SWP Bars           Length(2)    바만 표시 (텍스트 수치 없음)
│           │   ├── RAM line                        "RAM ▮▮▮▮▯▯" (segmented: used/cached/free)
│           │   └── SWP line                        "SWP ▮▮▯▯"
│           ├── Memory Legend          Length(2)    2줄 범례 (RAM 차트 바로 위)
│           │   ├── Line 1: "▮ used  ▮ cached  ▮ free  RAM {u}/{t} GiB ({p}%)"
│           │   └── Line 2: "{used}G/{cached}G/{free}G  avl:{avail}G"
│           └── RAM                    segmented chart Min(3)
│               └── 세그먼트 bar chart: used(Green/Yellow/Red) + cached(Blue), 매 tick 수직 막대
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
| RAM 차트 (Used 구간) | Green / Yellow / Red | 0-50% / 50-80% / 80%+ (used% 기준) |
| RAM 차트 (Cached 구간) | Blue | 커널 캐시/버퍼 (available - free) |
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

- MIG 인스턴스별 GPU Util, Mem Ctrl(메모리 컨트롤러), SM Util, VRAM 사용량 실시간 표시
- **Top Processes** — PID, 프로세스명, VRAM 사용량(MB) 기준 상위 5개 표시 (compute + graphics 프로세스 통합, VRAM 미지원 시 "N/A" 표시)
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
  - RAM 계산: `used = total - available` (비해제 가능), `cached = available - free` (해제 가능 캐시/버퍼), `free = MemFree`
- GPU Util / Mem Ctrl / **VRAM** / **PCIe** / CPU Total 시계열 sparkline 그래프 + **RAM 세그먼트 차트** (used/cached 색상 구분, 타이틀에 현재값 표시)
  - 모든 그래프 진행 방향 통일: **RightToLeft** — 최신 데이터가 오른쪽, 시간이 지남에 따라 왼쪽으로 이동 (RAM 세그먼트 차트와 동일)
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

# 디버그 모드 (NVML API 호출 로깅)
mig-gpu-mon --debug              # /tmp/mig-gpu-mon-debug.log에 기록

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

3단계: nvmlDeviceGetSamples(parent_handle, GPU_UTILIZATION_SAMPLES) — gpu_util 전용
    → 부모 GPU에서 raw GPU utilization 샘플 수집 (20ms 간격)
    → 최근 5개 샘플 평균, /10000으로 0-100% 스케일 변환
    → MIG 슬라이스 비율로 스케일링: mig_util = parent_util × total_slices / mig_slices
    → 예: parent=29%, MIG 3g.40gb → 29% × 7/3 ≈ 67%
    → 불가능 시: "N/A" 표시

4단계: nvmlDeviceGetSamples(parent_handle, MEMORY_UTILIZATION_SAMPLES) — memory_util 전용
    → 부모 GPU에서 memory controller utilization 샘플 수집
    → 3단계와 동일한 /10000 스케일링 + MIG 슬라이스 비율 환산
    → Ampere/Hopper 모든 아키텍처에서 Mem Ctrl 표시 가능
    → 또는 parent utilization_rates().memory 스케일링 (samples 미지원 드라이버 대응)
    → 모두 실패 시: "N/A" 표시
```

> **v0.3.16 변경:** GPM (`nvmlGpmMigSampleGet`) 코드를 전면 제거했다. 드라이버 535.129.03 + H100 MIG 환경에서 GPM 호출이 NVML 드라이버 내부 상태를 영구적으로 오염시켜 `memory_info()`가 `InUse` 에러를 반환하는 현상이 확인되었다. Startup Probe와 Phase 1.5 Post-probe 등 다중 방어 코드에도 불구하고, GPM 호출 자체가 세션 수준에서 NVML을 오염시키므로 근본적으로 제거하는 것이 유일한 해결책이었다. Mem Ctrl은 드라이버 550+ 업그레이드 시 `nvmlDeviceGetSamples(MEM_UTIL)` 또는 `utilization_rates()` 정상 동작으로 해결 가능하다.

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

> **v0.3.16:** GPM 수집 코드가 전면 제거되었다. `--debug` 플래그로 확인한 결과:
> - GPM Startup Probe 전: `memory_info()` = OK
> - GPM Startup Probe 후: `memory_info()` = FAIL (`InUse`) — 영구 오염
> - `nvmlDeviceGetSamples(MEM_UTIL)`: size query OK (72개) → data fetch `NOT_SUPPORTED` (ret=3)
> - parent `utilization_rates()`: FAIL
> GPM이 유일한 Mem Ctrl 소스였지만, VRAM 안정성을 위해 제거. 드라이버 550+ 권장.

| GPU 아키텍처 | Mem Ctrl 표시 (v0.3.16) | 비고 |
|---|---|---|
| Ampere (A100/A30) | "N/A" | nvmlDeviceGetSamples(MEM_UTIL) 미지원 |
| Hopper (H100) + 드라이버 535 | "N/A" | GPM 제거 (NVML 오염), samples NOT_SUPPORTED |
| Hopper (H100) + 드라이버 550+ | 표시 예상 | utilization_rates() 또는 samples 정상 동작 예상 |

> **구현 상태 (v0.3.16):** GPM 코드가 전면 제거되었다. `--debug` 플래그가 추가되어 모든 NVML API 호출의 성공/실패를 `/tmp/mig-gpu-mon-debug.log`에 기록할 수 있다.

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

#### Top Processes 누락 버그 분석 및 수정 (v0.3.2)

##### 증상

MIG 환경에서 "Top Processes" 패널에 "No compute processes"만 표시되고, 실제 실행 중인 프로세스가 보이지 않는 현상.

##### 근본 원인 분석 — 3가지 문제

**문제 1: `UsedGpuMemory::Unavailable` 프로세스 완전 제거**

```rust
// 수정 전: VRAM 정보 없으면 프로세스 자체를 제외
let mut entries: Vec<(u32, u64)> = procs
    .iter()
    .filter_map(|p| {
        let vram = match p.used_gpu_memory {
            UsedGpuMemory::Used(bytes) => bytes,
            UsedGpuMemory::Unavailable => return None,  // ← 프로세스 누락!
        };
        Some((p.pid, vram))
    })
    .collect();
```

MIG 환경(특히 드라이버 535.x)에서 NVML이 프로세스의 VRAM을 `Unavailable`로 반환하면, 해당 프로세스가 `filter_map`에서 완전히 제거되어 UI에 나타나지 않았다.

**문제 2: Compute 프로세스만 수집**

```rust
// 수정 전: compute 프로세스만 수집
let (process_count, top_processes) = match device.running_compute_processes() { ... };
// → graphics 프로세스 (Vulkan/OpenGL 등)는 수집되지 않음
```

`running_graphics_processes()`를 호출하지 않아, CUDA를 사용하지 않는 그래픽 프로세스가 누락되었다.

**문제 3: API 에러 시 무조건 빈 리스트**

```rust
// 수정 전: 에러 → 빈 리스트, 원인 파악 불가
Err(_) => (0, Vec::new()),
```

`running_compute_processes()` 실패 시 `(0, Vec::new())`를 반환하여, 프로세스가 실제로 없는 것처럼 표시되었다.

##### 수정 내용 (3건)

**수정 1: `GpuProcessInfo.vram_used` → `Option<u64>`** (`metrics.rs`)

```rust
// 수정 전: u64 — Unavailable 프로세스 배제
pub vram_used: u64,

// 수정 후: Option<u64> — Unavailable은 None으로 보존
pub vram_used: Option<u64>,

pub fn vram_used_mb(&self) -> Option<u64> {
    self.vram_used.map(|v| v / (1024 * 1024))
}
```

**수정 2: Compute + Graphics 프로세스 통합 수집** (`nvml.rs`)

```rust
// 수정 후: 양쪽 모두 수집, PID 기반 dedup
let mut seen_pids = HashSet::new();
let mut entries: Vec<(u32, Option<u64>)> = Vec::new();

// Compute processes (PyTorch, CUDA)
if let Ok(procs) = device.running_compute_processes() {
    for p in &procs { /* ... */ if seen_pids.insert(p.pid) { entries.push(...); } }
}
// Graphics processes (Vulkan, OpenGL)
if let Ok(procs) = device.running_graphics_processes() {
    for p in &procs { /* ... */ if seen_pids.insert(p.pid) { entries.push(...); } }
}
```

- 양쪽 API 모두 개별 `if let Ok` — 한쪽 실패해도 다른 쪽은 정상 수집
- `HashSet<u32>`로 PID 중복 방지 (양쪽에 동시 존재하는 프로세스)

**수정 3: VRAM "N/A" 표시** (`dashboard.rs`)

```rust
// 수정 후: VRAM 없으면 "N/A" 표시 (프로세스는 보존)
let vram_str = match proc.vram_used_mb() {
    Some(mb) => format!("{:>7} MB", mb),
    None => format!("{:>10}", "N/A"),
};
```

##### 정렬 로직 개선

```rust
// Known VRAM 내림차순 → Unavailable은 뒤로 → PID 기준 안정 정렬
entries.sort_by(|a, b| match (b.1, a.1) {
    (Some(bv), Some(av)) => bv.cmp(&av),
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (None, None) => a.0.cmp(&b.0),
});
entries.truncate(5);
```

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/gpu/metrics.rs` | `GpuProcessInfo.vram_used` → `Option<u64>`, `vram_used_mb()` → `Option<u64>` |
| `src/gpu/nvml.rs` | compute + graphics 통합 수집, PID dedup, `Unavailable` → `None` 보존, 개별 에러 처리 |
| `src/ui/dashboard.rs` | VRAM "N/A" 표시, "No processes" 메시지 업데이트 |

##### 상호 검증 매트릭스

| 시나리오 | 프로세스 표시 | VRAM 표시 | 검증 |
|----------|-------------|-----------|------|
| Compute 프로세스 + VRAM 정상 | 표시됨 | MB 값 | 기존 동작 유지 |
| Compute 프로세스 + VRAM Unavailable | 표시됨 | "N/A" | 수정 1: Option<u64> |
| Graphics 프로세스만 존재 | 표시됨 | MB 또는 "N/A" | 수정 2: graphics 추가 |
| 양쪽 동일 PID | 1건만 표시 | 중복 없음 | HashSet dedup |
| compute API 실패, graphics 정상 | Graphics만 표시 | MB 또는 "N/A" | 개별 if let Ok |
| 양쪽 모두 실패 | "No processes" | — | 빈 entries |
| 5개 초과 프로세스 | 상위 5개 (VRAM 기준) | MB 우선, N/A 후순위 | 정렬 로직 |
| MIG 인스턴스 | 해당 MIG 프로세스 | MB 또는 "N/A" | MIG 핸들에서 수집 |

#### MIG Top Processes 부모 디바이스 폴백 버그 분석 및 수정 (v0.3.3)

##### 증상

v0.3.2에서 `UsedGpuMemory::Unavailable` 프로세스 보존 및 compute+graphics 통합 수집을 적용했음에도, MIG 인스턴스에서 "No processes"가 계속 표시되는 현상.

##### 근본 원인 분석

**문제: `running_compute_processes()` / `running_graphics_processes()`가 MIG 디바이스 핸들에서 실패**

```rust
// collect_device_metrics() 내부 — MIG 핸들 전달
if let Ok(procs) = device.running_compute_processes() {   // ← MIG 핸들에서 Err 반환
    for p in &procs { ... }
}
if let Ok(procs) = device.running_graphics_processes() {  // ← MIG 핸들에서 Err 반환
    for p in &procs { ... }
}
```

NVIDIA 드라이버 535.x 등에서 `nvmlDeviceGetComputeRunningProcesses()` / `nvmlDeviceGetGraphicsRunningProcesses()`는 **MIG 디바이스 핸들**에 대해 `NVML_ERROR_NOT_SUPPORTED`를 반환한다. `if let Ok` 패턴으로 에러가 조용히 무시되어 `entries`가 항상 빈 상태 → "No processes" 표시.

반면 **부모 GPU 디바이스 핸들**에서 같은 API를 호출하면 모든 MIG 인스턴스의 프로세스가 `gpu_instance_id` 필드와 함께 정상 반환된다.

##### 수정 내용

**`collect_mig_instances()`에 Phase 3 추가** (`nvml.rs`)

```rust
// === Phase 3: MIG 프로세스 부모 디바이스 폴백 ===
// Phase 1에서 MIG 핸들로 수집 실패한 경우, 부모 GPU에서 프로세스 쿼리 후
// gpu_instance_id로 필터링하여 각 MIG 인스턴스에 분배

// 1. 부모 디바이스에서 compute + graphics 프로세스 1회 쿼리
let parent_procs = parent_device.running_compute_processes()
    + parent_device.running_graphics_processes();  // PID dedup

// 2. 각 MIG 인스턴스에 gpu_instance_id 매칭하여 분배
for (mig_handle, metrics) in &mut phase1 {
    if !metrics.top_processes.is_empty() { continue; }  // 이미 수집 성공이면 skip
    let gi_id = get_device_info(mig_device).gpu_instance_id;
    metrics.top_processes = parent_procs
        .filter(|p| p.gpu_instance_id == gi_id)
        .sort_by_vram_desc()
        .truncate(5);
}
```

**핵심 설계:**
- `nvml-wrapper 0.10`의 `ProcessInfo` 구조체가 `gpu_instance_id: Option<u32>` 필드 제공 → MIG 인스턴스별 필터링 가능
- Phase 1에서 이미 프로세스를 수집한 MIG 인스턴스는 건너뜀 (일부 드라이버에서는 MIG 핸들에서도 동작)
- 부모 디바이스 쿼리는 1회만 수행 → tick당 추가 NVML IPC 최소화

##### 3-phase 수집 전체 흐름

```
collect_mig_instances():
  Phase 1: 각 MIG 인스턴스의 기본 메트릭 수집 (VRAM, util, 프로세스 시도)
           → MIG 핸들에서 프로세스 API 실패 시 top_processes = []
  Phase 2: GPM 폴백 (memory_util, Hopper+ only)
           → 모든 VRAM 이미 수집 완료 → GPM 오염 무관
  Phase 3: 프로세스 부모 디바이스 폴백 (NEW)
           → 부모 GPU에서 프로세스 쿼리 → gpu_instance_id로 MIG 인스턴스별 분배
```

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/gpu/nvml.rs` | `collect_mig_instances()`에 Phase 3 추가 — 부모 디바이스 프로세스 쿼리 + `gpu_instance_id` 필터링 + MIG 인스턴스별 분배 |

##### 상호 검증 매트릭스

| 시나리오 | 프로세스 표시 | VRAM 표시 | 검증 |
|----------|-------------|-----------|------|
| MIG 핸들 프로세스 API 성공 | Phase 1에서 직접 수집 | MB 또는 "N/A" | `!top_processes.is_empty()` → Phase 3 skip |
| MIG 핸들 프로세스 API 실패 (535.x) | 부모 디바이스에서 수집 → gi_id 필터 | MB 또는 "N/A" | Phase 3 폴백 동작 |
| 부모 디바이스 프로세스 API도 실패 | "No processes" | — | 양쪽 모두 실패 시 빈 리스트 |
| 여러 MIG 인스턴스에 프로세스 분산 | 각 MIG별로 올바르게 분배 | MB 또는 "N/A" | `gpu_instance_id` 매칭으로 정확한 분배 |
| Non-MIG GPU | Phase 3 미실행 | MB 값 | `collect_mig_instances` 자체가 호출되지 않음 |

##### v0.3.2 → v0.3.3 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.2: VRAM Unavailable 보존 | 프로세스 VRAM `None`일 때 프로세스 자체는 유지 | `collect_device_metrics()` |
| v0.3.2: compute + graphics 통합 | 양쪽 API 모두 수집, PID dedup | `collect_device_metrics()` |
| **v0.3.3: 부모 디바이스 폴백** | **MIG 핸들 프로세스 API 실패 시 부모에서 수집 → gi_id 분배** | `collect_mig_instances()` Phase 3 |

v0.3.2는 "MIG 핸들에서 프로세스가 반환될 때 데이터 손실 방지", v0.3.3은 "MIG 핸들에서 프로세스 API 자체가 실패할 때 부모 디바이스로 폴백".

#### Top Processes 깜빡임 + 자원 감사 수정 (v0.3.4)

##### 증상

MIG 환경에서 Top Processes가 한 틱 나타났다가 사라지는 깜빡임 현상. 장기 운영 시 HashMap 프루닝 조건 버그로 orphan 엔트리 누적 가능.

##### 근본 원인 분석 — 3가지 이슈

**이슈 1: Top Processes carry-forward 없음**

`memory_used`는 API 실패 시 이전 값을 유지하는 carry-forward가 있지만, `top_processes`에는 동일 보호가 없었다.

```
Tick 1: MIG 디바이스 running_compute_processes() 성공 → 프로세스 표시
Tick 2: 같은 API 실패 → top_processes 빈 배열 → Phase 3 시도
Phase 3: gpu_instance_id 없으면 전부 필터 → "No processes" 표시
Tick 3: API 다시 성공 → 프로세스 깜빡임
```

**이슈 2: Phase 3 gpu_instance_id 필터 과도하게 엄격**

```rust
// 기존: gpu_instance_id가 None이면 모든 프로세스 필터됨
.filter(|(_, _, proc_gi)| *proc_gi == gi_id && gi_id.is_some())
```

부모 GPU의 프로세스에 `gpu_instance_id`가 설정되지 않은 드라이버에서는 Phase 3 폴백이 무용지물.

**이슈 3: app.rs history HashMap 프루닝 조건 버그**

```rust
// 기존: GPU 수가 동일하면 프루닝 안 됨 → MIG 재구성 시 orphan 누적
if self.history.len() > new_metrics.len() { ... }
```

4 MIG → 다른 UUID의 4 MIG 재구성 시 old 4개 + new 4개 = 8개 엔트리 누적.

##### 수정 내역 (5개 변경)

**Fix 1: Top Processes carry-forward** (`app.rs`)

```rust
// NVML 프로세스 API 간헐적 실패 시 이전 틱 프로세스 유지
if m.top_processes.is_empty() {
    if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) {
        if !prev.top_processes.is_empty() {
            m.top_processes = prev.top_processes.clone();
            m.process_count = prev.process_count;
        }
    }
}
```

**Fix 2: Phase 3 gpu_instance_id 폴백 완화** (`nvml.rs`)

```rust
// 부모 프로세스 중 아무도 gpu_instance_id가 없으면 전체 프로세스 표시
let any_gi_available = parent_procs.iter().any(|(_, _, gi)| gi.is_some());

let entries = if any_gi_available && gi_id.is_some() {
    // 정상 경로: gpu_instance_id 매칭 필터
    parent_procs.filter(|p| p.gpu_instance_id == gi_id)
} else {
    // 폴백: 전체 프로세스 표시 (아무것도 안 보이는 것보다 나음)
    parent_procs.all()
};
```

**Fix 3: HashMap 프루닝 정확도 개선** (`app.rs`)

```rust
// 기존: len 비교만 → MIG 재구성 시 UUID 변경 감지 불가
// 수정: UUID 불일치 시 항상 프루닝 + shrink_to() 추가
if self.history.len() != new_metrics.len()
    || self.history.keys().any(|uuid| !new_metrics.iter().any(|m| m.uuid == *uuid))
{
    self.history.retain(...);
    // 방어적 capacity 축소
    if self.history.capacity() > target * 2 {
        self.history.shrink_to(target);
    }
}
```

**Fix 4: proc_name_cache shrink 임계값 개선** (`nvml.rs`)

```rust
// 기존: len.max(16) * 4 → 1000 PID 사라져도 capacity 유지
// 수정: target * 2 → 더 적극적 메모리 회수
let target = name_cache.len().max(16) * 2;
if name_cache.capacity() > target * 2 {
    name_cache.shrink_to(target);
}
```

**Fix 5: datetime 포맷 캐시** (`dashboard.rs`)

```rust
// thread_local 캐시 — 초 단위 변경 시에만 재생성
thread_local! {
    static TIME_CACHE: RefCell<(i64, String)> = RefCell::new((0, String::new()));
}
// 같은 초 내 반복 호출 시 캐시 반환 → 틱당 String 할당 1회 절감
```

##### 수정 파일

| 파일 | 변경 내용 |
|------|----------|
| `src/app.rs` | Top Processes carry-forward + HashMap 프루닝 조건 수정 + shrink_to() 추가 |
| `src/gpu/nvml.rs` | Phase 3 gpu_instance_id 폴백 완화 + proc_name_cache shrink 임계값 개선 |
| `src/ui/dashboard.rs` | datetime 포맷 thread_local 캐시 |

##### 교차 검증 매트릭스

| 시나리오 | Top Processes 표시 | 자원 영향 | 검증 |
|----------|-------------------|----------|------|
| MIG 프로세스 API 간헐적 실패 | 이전 틱 값 유지 (carry-forward) | Clone 1회/틱 (5 프로세스) | API 실패 시에만 동작 |
| 부모 GPU에 gpu_instance_id 없음 | 전체 부모 프로세스 표시 | 기존과 동일 | 폴백 경로 활성화 |
| MIG 재구성 (UUID 변경, GPU 수 동일) | — | orphan 엔트리 즉시 제거 | UUID 불일치 감지 |
| 대량 PID 소멸 (1000→10) | — | capacity 적극적 축소 | target*2 임계값 |
| 1초 interval 장기 운영 | 정상 | RSS ~4-8MB 안정 유지 | 모든 버퍼 bounded |

##### v0.3.3 → v0.3.4 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.3: 부모 디바이스 폴백 | MIG 핸들 프로세스 API 실패 → 부모에서 수집 | `collect_mig_instances()` Phase 3 |
| **v0.3.4: carry-forward** | **Phase 3도 빈 결과 시 이전 틱 값 유지** | `app.rs:update_metrics()` |
| **v0.3.4: gi_id 폴백 완화** | **부모 프로세스에 gi_id 없을 때 전체 표시** | `nvml.rs` Phase 3 필터 |
| **v0.3.4: HashMap 프루닝 정확도** | **UUID 변경 감지 + shrink** | `app.rs:update_metrics()` |
| **v0.3.4: 자원 감사 최적화** | **proc_name_cache shrink + datetime 캐시** | `nvml.rs`, `dashboard.rs` |

v0.3.3은 "프로세스 수집 폴백 경로 추가", v0.3.4는 "폴백 경로 개선 + 표시 안정성 + 장기 운영 자원 최적화".

#### Sparkline 방향 버그 + 장기 운영 최적화 (v0.3.5)

##### 증상

1. Sparkline 그래프가 최신 데이터가 아닌 **가장 오래된 데이터**를 표시하며, 시간축 방향이 반전되어 있음
2. CPU sparkline의 f32→u64 변환 시 truncation으로 정밀도 손실 (99.7% → 99)
3. PCIe sparkline 타이틀에 TX/RX 모두 표시하지만 그래프는 TX만 렌더링 — 사용자 혼동
4. 프로세스명 캐시에서 매 tick `String::clone()` 발생 — 장기 운영 시 불필요한 힙 할당 누적
5. throttle_reasons가 매 tick `String` 할당 — "None" 등 빈번한 단일 값에도 힙 할당

##### 근본 원인 분석

**Bug 1: ratatui `RenderDirection::RightToLeft` 의미 오해 (Critical)**

ratatui의 `RightToLeft` 방향은 `data[0]`을 **오른쪽 끝**에 배치한다:
```
data = [0, 1, 2, 3, 4, 5, 6, 7, 8]
RightToLeft → "xxx█▇▆▅▄▃▂▁ "
//              data[8]←→data[0] (오른쪽 끝)
```

VecDeque에 `[oldest(0), ..., newest(N)]` 순서로 저장된 데이터를 그대로 전달하면:
- `data[0]` (가장 오래된 값)이 오른쪽 끝(최신 위치)에 표시됨
- `data[N]` (최신 값)이 왼쪽(과거 위치)에 표시됨
- `max_index = min(width, data.len())` 제한으로, 300개 히스토리 중 **가장 오래된 ~80개만** 렌더되고 최신 데이터는 아예 표시되지 않음

```
VecDeque: [T0(oldest), T1, T2, ..., T299(newest)]
Sparkline: data[0..80] = [T0, T1, ..., T79]  ← 가장 오래된 80개만!
화면:       T79 ← T78 ← ... ← T1 ← T0(오른쪽 끝)
```

**Bug 2: f32 truncation**

```rust
// Before: truncation (99.7% → 99)
buf.extend(src.iter().map(|&v| v as u64));
```

**Bug 3: PCIe 타이틀 모호성**

타이틀 `PCIe TX:12.3 RX:56.7 MB/s`이지만 sparkline은 `history.pcie_tx_kbps`만 사용 — RX 값 변화가 그래프에 반영 안 됨.

##### 수정 내용 (6개 변경)

**Fix 1: Sparkline 데이터 역순 변환** (`dashboard.rs`)

```rust
// Before: oldest → newest 순서 (data[0]=oldest → 오른쪽 끝)
buf.extend(src.iter().map(|&v| v as u64));

// After: newest → oldest 순서 (data[0]=newest → 오른쪽 끝)
buf.extend(src.iter().rev().map(|&v| v as u64));
```

`.rev()` 추가로 데이터 순서 반전:
- `data[0]` = newest → 오른쪽 끝 ✓
- 터미널 폭(~80) 내 최신 데이터만 표시 ✓
- 버퍼 미충전 시 오른쪽부터 채워짐 ✓

**Fix 2: f32 rounding** (`dashboard.rs`)

```rust
// Before: truncation
buf.extend(src.iter().rev().map(|&v| v as u64));
// After: rounding
buf.extend(src.iter().rev().map(|&v| v.round() as u64));
```

**Fix 3: PCIe 타이틀 명확화** (`dashboard.rs`)

```rust
// Before: "PCIe TX:12.3 RX:56.7 MB/s" — 그래프가 TX+RX인 듯 오해
// After: "PCIe TX:12.3 / RX:56.7 MB/s" + 기본 타이틀 "PCIe TX"
```

**Fix 4: `GpuProcessInfo::name` → `Rc<str>`** (`metrics.rs`, `nvml.rs`)

```rust
// Before: 매 tick String::clone() (힙 복사)
pub name: String,
fn process_name(&self, pid: u32) -> String { cache.get(&pid).clone() }

// After: Rc<str> clone = 포인터 카운트 증가만 (힙 할당 0)
pub name: Rc<str>,
fn process_name(&self, pid: u32) -> Rc<str> { cache.get(&pid).clone() }
```

**Fix 5: `throttle_reasons` → `Cow<'static, str>`** (`metrics.rs`, `nvml.rs`)

```rust
// Before: 매 tick String 할당 (빈번한 "None" 포함)
pub throttle_reasons: Option<String>,
fn format_throttle_reasons(tr) -> String { String::from("None") }

// After: 단일 플래그 fast path → Cow::Borrowed (제로 할당)
pub throttle_reasons: Option<Cow<'static, str>>,
fn format_throttle_reasons(tr) -> Cow<'static, str> {
    // "None", "Idle", "SwPwrCap", "HW-Slow", "SW-Therm", "HW-Therm" → Borrowed
    // 복합 플래그만 Cow::Owned 할당
}
```

**Fix 6: `unused import: Text` warning 제거** (`dashboard.rs`)

##### 수정 파일

| 파일 | 변경 내용 |
|------|----------|
| `src/ui/dashboard.rs` | Sparkline 데이터 `.rev()` 역순 변환, f32 rounding, PCIe 타이틀 명확화, unused import 정리 |
| `src/gpu/metrics.rs` | `GpuProcessInfo::name` → `Rc<str>`, `throttle_reasons` → `Cow<'static, str>` |
| `src/gpu/nvml.rs` | `proc_name_cache` → `HashMap<u32, Rc<str>>`, `process_name()` → `Rc<str>` 반환, `format_throttle_reasons()` → `Cow<'static, str>` 반환 + 단일 플래그 fast path |

##### 교차 검증 매트릭스

| 시나리오 | Sparkline 표시 | 성능 영향 | 검증 |
|----------|---------------|----------|------|
| 히스토리 300개, 터미널 80칸 | 최신 80개 표시 (newest → right) | 변경 없음 | `.rev()` + `RightToLeft` |
| 히스토리 10개, 터미널 80칸 | 오른쪽 끝부터 10개 채움 | 변경 없음 | `RightToLeft` 특성 유지 |
| CPU 99.7% | sparkline에 100 표시 | 변경 없음 | `.round()` 적용 |
| PCIe TX만 그래프 | 타이틀 "PCIe TX:" 명시 | 변경 없음 | 타이틀에서 혼동 제거 |
| throttle "None" (90%+ 빈도) | 동일 표시 | **String 할당 제거** | `Cow::Borrowed` |
| throttle "SwPwrCap, HW-Therm" | 동일 표시 | 기존과 동일 (Cow::Owned) | 복합 플래그 fallback |
| 프로세스명 캐시 히트 | 동일 표시 | **String clone → Rc bump** | GPU당 5개 × tick |
| top_processes carry-forward | 동일 표시 | **Vec<GpuProcessInfo> clone 비용 감소** | Rc<str> 이므로 name 복사 0 |

##### v0.3.4 → v0.3.5 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.4: 자원 감사 최적화 | proc_name_cache shrink + datetime 캐시 | `nvml.rs`, `dashboard.rs` |
| **v0.3.5: Sparkline 방향 수정** | **가장 오래된 데이터가 최신으로 표시되는 치명적 버그 수정** | `dashboard.rs` 전체 sparkline |
| **v0.3.5: 프로세스명 Rc<str>** | **매 tick String 힙 복사 제거 → 포인터 카운트만** | `metrics.rs`, `nvml.rs` |
| **v0.3.5: throttle Cow<'static, str>** | **빈번한 단일 값에 대한 힙 할당 제거** | `metrics.rs`, `nvml.rs` |

v0.3.4는 "장기 운영 자원 회수 최적화", v0.3.5는 "실시간 표시 정확성 수정 + tick당 반복 힙 할당 제거".

#### VRAM 실시간 반영 실패 + 장기 운영 자원 최적화 (v0.3.6)

##### 증상

- GPU 사용량이 많다가 감소했는데 VRAM 표시가 실시간으로 변경사항을 반영하지 못함
- Top Processes 패널에 이미 종료된 프로세스의 오래된 VRAM 값이 무기한 표시됨
- `memory_info()` 간헐 실패 시 이전의 높은 VRAM 값이 영구적으로 carry-forward됨

##### 근본 원인 분석 — 3가지 연쇄 버그

**버그 1: 프로세스 carry-forward 무기한 유지 (가장 핵심)**

```
위치: app.rs update_metrics()
```

GPU 사용량 감소로 프로세스가 정상 종료되면 `running_compute_processes()`가 빈 리스트를 반환한다. 그러나 코드는 "빈 리스트 = NVML 쿼리 깜빡임(flicker)"으로 간주하여 이전 tick의 오래된 프로세스 목록을 **만료 체크 없이 무한정 carry-forward**했다. 새 프로세스가 나타나기 전까지 죽은 프로세스의 VRAM 값이 영구 표시됨.

**버그 2: VRAM carry-forward 무기한 유지**

```
위치: app.rs update_metrics()
```

MIG 환경에서 `device.memory_info()`가 GPM 상태 corruption으로 간헐적으로 실패할 때, `memory_used = None` → 이전 tick의 높은 VRAM 값을 만료 없이 복사. VRAM이 10GB→2GB로 줄어도 실패 tick에서는 10GB가 유지됨.

**버그 3: Sparkline 히스토리 이중 강화**

```
위치: metrics.rs push_or_repeat()
```

`memory_used`가 `None`일 때 마지막 높은 값을 반복 기록하여 sparkline이 VRAM 감소를 반영하지 못하고 평탄한 높은 선으로 유지됨.

##### 수정 내용 (6개 변경)

**변경 1: 프로세스 carry-forward → PID 생존 확인**

```rust
// 이전: 무조건 이전 tick 프로세스 복사 (무기한)
if m.top_processes.is_empty() {
    m.top_processes = prev.top_processes.clone();
}

// 이후: /proc/{pid} 존재 여부로 살아있는 프로세스만 carry-forward
let alive: Vec<_> = prev.top_processes.iter()
    .filter(|p| {
        buf.clear();
        write!(buf, "/proc/{}", p.pid);
        Path::new(buf.as_str()).exists()
    })
    .cloned().collect();
```

- 프로세스 종료 즉시 반영 — TTL 임의값 문제 없음
- `/proc/{pid}` stat syscall은 커널 버퍼링으로 ~1μs 수준

**변경 2: VRAM carry-forward → TTL 3회 제한**

```rust
// 이전: memory_info() 실패 시 무조건 이전 값 복사 (무기한)
// 이후: 연속 3회까지만 carry-forward, 초과 시 None → "N/A"
const VRAM_CARRY_FORWARD_TTL: u32 = 3;

let count = if let Some(c) = self.vram_fail_count.get_mut(&m.uuid) {
    *c += 1; *c
} else {
    self.vram_fail_count.insert(m.uuid.clone(), 1); 1
};
if count <= VRAM_CARRY_FORWARD_TTL { /* carry forward */ }
// else: UI shows "N/A"
```

- 기본 1초 interval × 3회 = 3초 tolerance (일시적 flicker 커버)
- 성공 시 카운터 즉시 리셋

**변경 3: /proc/{pid} 경로 버퍼 재사용**

```rust
// 이전: 매 PID마다 format!("/proc/{}", pid) → String 힙 할당
// 이후: App 구조체에 proc_path_buf: String 재사용
buf.clear();
write!(buf, "/proc/{}", p.pid);  // 기존 버퍼 재사용, 할당 0
```

- tick당 25+ String 할당 제거 (GPU 5 × 프로세스 5)
- 300시간 기준 ~27억 회 불필요한 할당 방지

**변경 4: active_handles Vec → HashSet**

```rust
// 이전: Vec<usize> → cache.retain(|k, _| active_handles.contains(k))  // O(n) per entry
// 이후: HashSet<usize> → contains() O(1)
active_handles: RefCell<HashSet<usize>>,
```

- `prune_stale_caches()` 복잡도: O(n²) → O(n)
- MIG 128개(16 GPU × 8) 시 16,384 비교 → 128 해시 조회

**변경 5: history 정리 UUID HashSet 사전 구축**

```rust
// 이전: self.history.keys().any(|uuid| !new_metrics.iter().any(...))  // O(n×m)
// 이후: HashSet 사전 구축 → O(1) lookup
let uuid_set: HashSet<&Rc<str>> = new_metrics.iter().map(|m| &m.uuid).collect();
self.history.retain(|uuid, _| uuid_set.contains(uuid));
```

- 이중 중첩 `.any()` O(n×m) → HashSet O(n) 단일 순회

**변경 6: vram_fail_count entry() Rc clone 회피**

```rust
// 이전: self.vram_fail_count.entry(m.uuid.clone()).or_insert(0)  // 매번 Rc clone
// 이후: get_mut/insert 패턴 → cache hit 시 clone 0
let count = if let Some(c) = self.vram_fail_count.get_mut(&m.uuid) {
    *c += 1; *c
} else {
    self.vram_fail_count.insert(m.uuid.clone(), 1); 1
};
```

##### 수정 파일

| 파일 | 변경 | 목적 |
|------|------|------|
| `app.rs` | PID 생존 확인 + VRAM TTL + proc_path_buf + UUID HashSet + Rc clone 회피 | VRAM 실시간 반영 + 자원 최적화 |
| `nvml.rs` | `active_handles` Vec→HashSet + 시그니처 변경 | prune O(n²)→O(n) |

##### 교차 검증 매트릭스

| 시나리오 | 검증 항목 | 기대 결과 |
|---------|----------|----------|
| GPU 사용량 감소 → 프로세스 종료 | Top Processes 패널 | 종료된 프로세스 즉시 사라짐 |
| memory_info() 1-3회 연속 실패 | VRAM 게이지 | 이전 값 유지 (tolerance) |
| memory_info() 4회+ 연속 실패 | VRAM 게이지 | "N/A" 표시 |
| memory_info() 실패 후 성공 | VRAM 게이지 + 카운터 | 즉시 실제 값 반영, 카운터 리셋 |
| MIG 재구성 반복 100회 | active_handles HashSet | prune O(n), 메모리 일정 |
| 128 MIG 인스턴스 | history 정리 | O(n) HashSet lookup (기존 O(n×m)=16K 비교 제거) |
| 300시간 장기 운영 | proc_path_buf | String 할당 0 (버퍼 재사용) |
| vram_fail_count 정상 tick | Rc clone | get_mut hit → clone 0 |
| GPU 제거 | vram_fail_count | retain()으로 함께 정리 |

##### v0.3.5 → v0.3.6 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.5: Sparkline 방향 + 힙 최적화 | sparkline 정확성 + Rc/Cow 최적화 | `dashboard.rs`, `metrics.rs`, `nvml.rs` |
| **v0.3.6: PID 생존 확인** | **프로세스 carry-forward 즉시 만료** | `app.rs` 프로세스 표시 |
| **v0.3.6: VRAM TTL** | **memory_info() 실패 시 3회 제한** | `app.rs` VRAM 표시 |
| **v0.3.6: active_handles HashSet** | **prune O(n²)→O(n)** | `nvml.rs` 캐시 정리 |
| **v0.3.6: UUID HashSet** | **history 정리 O(n×m)→O(n)** | `app.rs` GPU 제거 감지 |

v0.3.5는 "sparkline 정확성 + tick당 힙 최적화", v0.3.6은 "VRAM 실시간 반영 수정 + carry-forward 안전성 + 알고리즘 복잡도 개선".

#### VRAM N/A 전환 + 장기 운영 메모리 최적화 (v0.3.8)

##### 증상

- VRAM이 처음에는 정상 표시되다가 수 초 후 **"N/A"로 영구 전환**됨
- MIG 환경에서 GPM(Hopper+) 사용 시에만 발생
- 128코어 시스템에서 CPU 코어 바 렌더링 시 tick당 ~384회 String 할당

##### 근본 원인 분석 — 2가지 문제

**문제 1: GPM cross-tick NVML 상태 오염 (핵심)**

```
위치: nvml.rs collect_mig_instances() Phase 2
```

`nvmlGpmMigSampleGet()` 호출이 NVML 드라이버 내부 상태를 오염시키며, 이 오염이 **다음 tick까지 지속**되어 `memory_info()` 호출이 영구 실패한다.

```
Tick 1: Phase 1 memory_info() 성공 → Phase 2 GPM 호출 → 드라이버 상태 오염
Tick 2: Phase 1 memory_info() 실패 (이전 tick 오염) → carry-forward 시작
Tick 4: VRAM_CARRY_FORWARD_TTL(3) 초과 → memory_used = None → "N/A" 고정
```

2-phase 설계는 **같은 tick 내**에서는 VRAM을 보호하지만, **이전 tick Phase 2의 오염이 다음 tick Phase 1에 영향**을 주는 cross-tick 문제는 방어하지 못했다.

**문제 2: 장기 운영 시 불필요한 per-tick 할당**

| 항목 | 할당 패턴 | 영향 |
|------|----------|------|
| MIG display name | 매 tick `format!().into()` → 새 `Rc<str>` | MIG 인스턴스 수 × tick |
| `phase1` Vec | `Vec::new()` → 매 tick realloc | MIG 수집 시 |
| `seen_pids` HashSet | `HashSet::new()` → 매 tick per-device alloc | 디바이스 수 × tick |
| CPU 바 문자열 | `make_bar()` → 코어 수 × `String` 할당 | 128코어 = 128 alloc/draw |
| 시간 문자열 | `TIME_CACHE.clone()` + `format!()` | 2 alloc/draw |

##### 수정 내용 (8개 변경)

**변경 1: GPM MIG Phase 2 완전 제거** (`nvml.rs`)

```rust
// 제거: Phase 2 GPM fallback 블록 전체
// nvmlGpmMigSampleGet() 호출 → NVML 상태 오염 원천 차단
// memory_util은 Fallback 1(process utilization)에서 수집 가능한 경우만 표시
```

GPM으로 얻는 `memory_util`(DRAM BW) 하나 때문에 핵심 메트릭인 VRAM을 잃는 트레이드오프가 맞지 않으므로, MIG 환경에서 GPM Phase 2를 완전 스킵.

**변경 2: MIG display name 캐싱** (`nvml.rs`)

```rust
// Before: 매 tick 새 Rc<str> 생성
metrics.name = format!("MIG {mig_idx} (GPU {gpu_index}: {})", metrics.name).into();
// After: DeviceInfo.mig_display_name 캐시, Rc::clone()만
let cached = device_cache.get(&key).and_then(|i| i.mig_display_name.clone());
metrics.name = cached.unwrap_or_else(|| { /* format + cache + return */ });
```

**변경 3: `phase1` Vec 사전 할당** (`nvml.rs`)

```rust
// Before: Vec::new() → push마다 realloc 가능
// After: Vec::with_capacity(max_count) → 1회 할당
```

**변경 4: PID dedup HashSet 재사용** (`nvml.rs`)

```rust
// Before: 매 tick per-device HashSet::new()
// After: NvmlCollector.proc_seen_pids: RefCell<HashSet<u32>> 재사용
//        + parent_procs/entries Vec::with_capacity(16)
```

**변경 5: 시간 문자열 zero-alloc 렌더링** (`dashboard.rs`)

```rust
// Before: TIME_CACHE에서 c.1.clone() + format!(" {} ", now) → 2 alloc/draw
// After: 렌더링을 TIME_CACHE.with 클로저 내부로 이동
//        + write!(c.1, ...) 버퍼 재사용 + c.1.as_str() 참조 → 0 alloc/draw
```

**변경 6: CPU 바 룩업 테이블** (`dashboard.rs`)

```rust
// Before: make_bar(usage, bar_width) → 코어당 String 할당 (128코어 = 128 alloc)
// After: BAR_TABLE thread_local! 룩업 테이블
//        bar_width+1개 패턴 사전 빌드, bt.1[filled].as_str() 참조
//        터미널 리사이즈 시에만 재빌드 → 0 alloc/draw (첫 draw 이후)
```

**변경 7: thread_local const 초기화** (`dashboard.rs`)

```rust
// clippy 권장: thread_local! 초기화자에 const 사용
static TIME_CACHE: ... = const { RefCell::new(...) };
static BAR_TABLE: ... = const { RefCell::new(...) };
```

**변경 8: entries/parent_procs 사전 할당** (`nvml.rs`)

```rust
// Before: Vec::new() → 첫 push 시 alloc
// After: Vec::with_capacity(16) → process count에 맞는 초기 할당
```

##### 수정 파일

| 파일 | 변경 | 관련 변경 |
|------|------|----------|
| `src/gpu/nvml.rs` | GPM Phase 2 제거 + MIG name 캐싱 + Vec/HashSet 재사용 | 변경 1, 2, 3, 4, 8 |
| `src/ui/dashboard.rs` | 시간 문자열 zero-alloc + BAR_TABLE 룩업 + const 초기화 | 변경 5, 6, 7 |

##### 교차 검증 매트릭스

| 검증 항목 | 방법 | 기대 결과 |
|----------|------|----------|
| VRAM N/A 전환 | MIG + Hopper 환경에서 장시간 실행 | VRAM 값 유지, N/A 전환 없음 |
| memory_util 표시 | Fallback 1(process util) 가용 시 | 정상 표시 (불가 시 N/A) |
| MIG name 캐싱 | 2+ tick 실행 후 DeviceInfo 캐시 확인 | 첫 tick만 format, 이후 Rc::clone |
| CPU 바 할당 | 128코어 시스템 draw | 첫 draw 후 bar String 할당 0 |
| 시간 문자열 | draw_header per-frame | clone/format 할당 없음 |
| cargo clippy | 경고 확인 | 신규 경고 없음 |

##### v0.3.6 → v0.3.8 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.6: VRAM TTL (3회) | memory_info() 실패 시 carry-forward 제한 | `app.rs` VRAM 표시 |
| **v0.3.8: GPM MIG 비활성화** | **cross-tick NVML 상태 오염 원천 차단** | `nvml.rs` MIG 수집 |
| **v0.3.8: MIG name 캐싱** | **per-tick Rc<str> 할당 제거** | `nvml.rs` DeviceInfo 캐시 |
| **v0.3.8: PID dedup 재사용** | **per-tick HashSet 할당 제거** | `nvml.rs` 프로세스 수집 |
| **v0.3.8: BAR_TABLE 룩업** | **per-draw 128+ String 할당 제거** | `dashboard.rs` CPU 코어 바 |
| **v0.3.8: 시간 문자열 zero-alloc** | **per-draw 2 String 할당 제거** | `dashboard.rs` 헤더 |

v0.3.6은 "VRAM TTL로 정체 방지", v0.3.8은 "GPM 오염 원천 차단 + 장기 운영 per-tick 할당 최소화".

#### RAM 세그먼트 차트 시각적 갭 수정 (v0.3.9)

##### 증상

- RAM 세그먼트 차트에서 녹색(used)과 파란색(cached) 영역이 **분리되어 표시**됨
- 녹색 비중이 줄어들어도 파란색이 녹색에 밀착되지 않고, 두 영역 사이에 빈 공간 발생
- 특히 used 비율이 낮을 때(< 20%) 갭이 두드러짐

##### 근본 원인 분석

```
위치: dashboard.rs draw_ram_segmented_chart()
```

used(녹색)가 fractional block 문자(`▁▂▃▄▅▆▇`)로 끝나는 셀에서, 해당 문자는 셀의 **아래쪽 일부만** 채운다. 셀의 나머지 위쪽 부분은 배경색(검정)으로 남아 빈 공간이 된다. cached(파란색)는 그 **다음 셀**부터 `█`(전체 블록)으로 시작하므로, 녹색 fractional 셀의 빈 위쪽 공간이 시각적 갭으로 보인다.

```
수정 전 (갭 발생):
│█│ ← cached (blue, full block)
│▃│ ← used (green, 3/8 block) — 위쪽 5/8이 빈 배경색
│█│ ← used (green, full block)

수정 후 (밀착):
│█│ ← cached (blue, full block)
│▃│ ← used (green fg, blue bg) — 아래 3/8 녹색, 위 5/8 파란색
│█│ ← used (green, full block)
```

##### 수정 내용 (1개 변경)

**변경 1: fractional used 셀에 cached 배경색 적용** (`dashboard.rs`)

```rust
// Before: fg만 설정, bg는 기본(검정)
} else if bottom_row == used_rows && used_frac > 0.05 {
    (bar_chars[(used_frac * 8.0) as usize % 8], used_color)

// After: fg=used_color, bg=Color::Blue (cached 존재 시)
} else if bottom_row == used_rows && used_frac > 0.05 {
    let bg = if has_cached { Color::Blue } else { Color::Reset };
    (bar_chars[(used_frac * 8.0) as usize % 8], used_color, bg)
```

하나의 셀 안에서 하단(fg) = 녹색 used, 상단(bg) = 파란색 cached를 동시에 표현하여 시각적 연속성을 보장한다.

##### 수정 파일

| 파일 | 변경 | 관련 변경 |
|------|------|----------|
| `src/ui/dashboard.rs` | fractional used 셀 bg 색상 + 3-tuple 반환 구조 | 변경 1 |

##### 교차 검증 매트릭스

| 검증 항목 | 방법 | 기대 결과 |
|----------|------|----------|
| used+cached 밀착 | used 10-30% + cached 50%+ 시나리오 | 녹색-파란색 사이 갭 없음 |
| used 0% (cached만) | cached만 존재 시 | 파란색만 하단부터 표시 |
| cached 0% (used만) | used만 존재 시 | 녹색만 하단부터, bg=Reset |
| 100% 사용 | used + cached = 100% | 차트 전체 채움, 빈 공간 없음 |
| cargo clippy | 경고 확인 | 신규 경고 없음 |

##### v0.3.8 → v0.3.9 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.8: GPM MIG 비활성화 + per-tick 할당 최소화 | cross-tick 오염 차단 + 장기 운영 메모리 | `nvml.rs`, `dashboard.rs` |
| **v0.3.9: RAM 차트 fractional 셀 bg 색상** | **used-cached 경계 시각적 갭 제거** | `dashboard.rs` RAM 세그먼트 차트 |

v0.3.8은 "데이터 수집 안정성 + 할당 최적화", v0.3.9는 "RAM 차트 시각적 정확성 수정".

#### MIG Mem Ctrl GPM 복원 + per-tick 중복 제거 (v0.3.10)

##### 증상

- MIG 인스턴스에서 Mem Ctrl.(Memory Controller Utilization) 값이 **항상 "N/A"**로 표시됨
- v0.3.8 이전에는 GPM을 통해 1~2초 지연 후 값이 정상 표시되었음
- GPU Util, VRAM은 정상 동작하지만 Mem Ctrl.만 수집 경로가 없음

##### 근본 원인 분석

```
위치: nvml.rs collect_mig_instances()
```

v0.3.8에서 GPM(`nvmlGpmMigSampleGet`)이 NVML 상태를 cross-tick 오염시켜 VRAM "N/A" 문제를 유발한다는 것을 발견하고, GPM Phase 2를 **전면 제거**했다. 그러나 이로 인해 MIG memory_util을 수집할 수 있는 모든 경로가 차단되었다:

```
수집 경로 분석 (v0.3.8~v0.3.9):
1. utilization_rates()       → MIG에서 항상 NVML_ERROR_NOT_SUPPORTED
2. GPM fallback (collect_device_metrics) → !is_mig 조건으로 차단
3. GPM fallback (collect_mig_instances) → v0.3.8에서 삭제됨
4. process utilization       → 프로세스 없거나 mem=0이면 None
→ 결과: memory_util 수집 경로 전무 → 항상 "N/A"
```

**핵심 통찰**: GPM이 NVML 상태를 오염시키지만, `memory_info()`는 Phase 1에서 **이미 수집 완료**된 상태. Phase 1 이후에 GPM을 호출하면 VRAM 데이터는 안전하다.

##### 수정 내용 (4개 변경)

**변경 1: Phase 1.5 GPM DRAM BW Util 복원** (`nvml.rs`)

```rust
// Phase 1: 모든 MIG 인스턴스의 VRAM(memory_info()) 수집 완료
// Phase 1.5 (신규): GPM으로 memory_util 수집 — VRAM은 이미 안전
let parent_info = self.get_device_info(parent_device, false);
if parent_info.gpm_supported {
    for (mig_handle, gi_id, metrics) in &mut phase1 {
        if metrics.memory_util.is_some() { continue; }
        if let Some(gi_id) = gi_id {
            let gpm_val = self.get_dram_bw_util_gpm(
                *mig_handle, true, Some(*gi_id), Some(parent_handle),
            );
            if let Some(val) = gpm_val {
                metrics.memory_util = Some(val);
            }
        }
    }
}
// Phase 2: 프로세스 수집 (기존)
```

Phase 1 → 1.5 → 2 순서로 실행되므로, GPM 호출 시점에 `memory_info()`는 이미 완료된 상태.
첫 tick은 `None` (delta 계산을 위한 이전 샘플 필요), 두 번째 tick부터 Mem Ctrl. 값 표시.

**변경 2: `gpu_instance_id` Phase 1 캐싱으로 중복 Device::new 제거** (`nvml.rs`)

```rust
// Before: Phase 1, 1.5, 2에서 각각 Device::new() + get_device_info() = 3회/인스턴스
// phase1: Vec<(nvmlDevice_t, GpuMetrics)>
let mig_device = Device::new(*mig_handle, &self.nvml);     // Phase 1.5
let mig_info = self.get_device_info(&mig_device, true);    // Phase 1.5
let mig_device = Device::new(*mig_handle, &self.nvml);     // Phase 2
let mig_info = self.get_device_info(&mig_device, true);    // Phase 2

// After: Phase 1에서 gi_id 캐싱, 1.5/2에서 직접 사용 = 1회/인스턴스
// phase1: Vec<(nvmlDevice_t, Option<u32>, GpuMetrics)>
let mig_info = self.get_device_info(&mig_device, true);    // Phase 1에서 1회
let gi_id = mig_info.gpu_instance_id;                      // 캐싱
// Phase 1.5/2에서 gi_id 직접 참조 — Device::new/get_device_info 불필요
```

MIG 7개 인스턴스 기준 tick당 HashMap lookup 14회 → 0회 절감.

**변경 3: Fallback 2에서 `get_device_info` 재호출 제거** (`nvml.rs`)

```rust
// Before: Fallback 2에서 get_device_info(&mig_device, true) 재호출
if metrics.gpu_util.is_none() {
    let mig_info = self.get_device_info(&mig_device, true);  // 중복!
    if let Some(mig_slices) = mig_info.gpu_instance_slice_count { ... }
}

// After: Phase 1 상단에서 이미 가져온 mig_info 재사용
if metrics.gpu_util.is_none() {
    if let Some(mig_slices) = mig_info.gpu_instance_slice_count { ... }
}
```

**변경 4: app.rs HashSet 이중 할당 제거** (`app.rs`)

```rust
// Before: 조건 체크용 + retain용 = 2회 HashSet 생성
if self.history.len() != new_metrics.len()
    || {
        let uuid_set: HashSet<_> = new_metrics.iter().map(|m| &m.uuid).collect();  // 1회
        self.history.keys().any(|uuid| !uuid_set.contains(uuid))
    }
{
    let uuid_set: HashSet<_> = new_metrics.iter().map(|m| &m.uuid).collect();  // 2회 (동일)
    self.history.retain(|uuid, _| uuid_set.contains(uuid));
}

// After: 1회 생성 후 조건 체크 + retain 모두에 사용
let uuid_set: HashSet<_> = new_metrics.iter().map(|m| &m.uuid).collect();  // 1회만
if self.history.len() != uuid_set.len()
    || self.history.keys().any(|uuid| !uuid_set.contains(uuid))
{
    self.history.retain(|uuid, _| uuid_set.contains(uuid));
}
```

##### 수정 파일

| 파일 | 변경 | 관련 변경 |
|------|------|----------|
| `src/gpu/nvml.rs` | Phase 1.5 GPM 복원 + gi_id 캐싱 + Fallback 2 중복 제거 | 변경 1, 2, 3 |
| `src/app.rs` | HashSet 이중 할당 제거 | 변경 4 |

##### 교차 검증 매트릭스

| 검증 항목 | 방법 | 기대 결과 |
|----------|------|----------|
| Mem Ctrl. 표시 | MIG + Hopper 환경에서 2+ tick 실행 | 첫 tick "N/A" → 두 번째 tick부터 0-100% 표시 |
| VRAM 안전성 | 장시간 실행 | Phase 1에서 수집 완료 후 GPM 호출 → VRAM "N/A" 전환 없음 |
| GPU Util 정상 | Fallback 1/2 경로 | 기존 동작 유지 |
| gi_id 캐싱 | Phase 1.5/2에서 Device::new 미호출 | HashMap lookup 0회 (캐시된 gi_id 직접 사용) |
| HashSet 단일 할당 | GPU reconfig 시나리오 | 동일 HashSet 1회만 생성 |
| Ampere 이전 GPU | GPM 미지원 환경 | Mem Ctrl. "N/A" 유지 (GPM 미지원 → gpm_supported=false → 스킵) |
| cargo clippy | 경고 확인 | 신규 경고 없음 |

##### 자원 누수 감사 결과

| 자원 | 보호 메커니즘 | 상태 |
|------|-------------|------|
| MetricsHistory VecDeque | `push_ring()` + max_entries cap | ✓ bounded |
| device_cache HashMap | `prune_stale_caches()` 매 tick | ✓ pruned |
| proc_name_cache | 매 tick active PID만 retain + shrink_to | ✓ pruned |
| gpm_prev_samples | stale 엔트리 prune 시 `nvmlGpmSampleFree` | ✓ freed |
| proc_sample_buf / sample_buf | grow-only BUT `capacity > floor*8` 시 shrink | ✓ bounded |
| App history/vram_fail_count | UUID 기반 retain + shrink_to | ✓ pruned |

모든 캐시/버퍼는 정상 prune되며, 장기 운영 시 unbounded growth 없음 확인.

##### v0.3.9 → v0.3.10 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.9: RAM 차트 fractional 셀 bg 색상 | used-cached 경계 시각적 갭 제거 | `dashboard.rs` RAM 세그먼트 차트 |
| **v0.3.10: Phase 1.5 GPM 복원** | **Phase 1 이후 안전한 GPM 호출로 Mem Ctrl. 수집 복원** | `nvml.rs` MIG 수집 Phase 1.5 |
| **v0.3.10: gi_id 사전 캐싱** | **per-tick Device::new + get_device_info 중복 제거** | `nvml.rs` Phase 1 → 1.5/2 |
| **v0.3.10: HashSet 단일 할당** | **GPU reconfig 시 이중 HashSet 생성 제거** | `app.rs` history 정리 |

v0.3.9는 "RAM 차트 시각적 정확성 수정", v0.3.10은 "Mem Ctrl. GPM 안전 복원 + per-tick 중복 작업 제거".

#### v0.3.11 — VRAM 추적 안정성 + GPM 자원 누수 수정 + 장기 운영 최적화

##### 문제 현상

1. **VRAM 추적 3초 후 멈춤**: MIG 환경에서 `memory_info()` 초반 몇 초 수집 후 "N/A"로 전환, 스파크라인 동결
2. **VRAM 그래프 급격한 스케일 점프**: VRAM "N/A" 전환 시 `vram_max`가 1MB로 붕괴 → 기존 히스토리 렌더링 왜곡
3. **GPM 샘플 메모리 누수**: MIG에서 `parent_handle?`/`gpu_instance_id?` 조기 리턴 시 할당된 샘플 미해제
4. **NvmlCollector Drop double-free**: `prune_stale_caches()`에서 free 후 Drop에서 재 free

##### 근본 원인 분석

Phase 1.5의 `nvmlGpmMigSampleGet()`이 NVML 드라이버 상태를 오염시켜 **다음 tick**의 Phase 1 `memory_info()`가 실패. 첫 tick은 GPM 이전 상태이므로 성공하지만, 2번째 tick부터 GPM 후유증으로 연속 실패 시작 → 3초 carry-forward 만료 → 이중 TTL(app.rs + metrics.rs) 동시 만료로 VRAM 완전 멈춤.

동시에 `memory_total`도 `None` → `dashboard.rs`의 `.unwrap_or(1)` → `vram_max=1MB` → 기존 수천 MB 히스토리가 스케일 왜곡.

##### 수정 내용 (7개 변경)

**변경 1: VRAM carry-forward TTL 3→10 확대** (`app.rs`, `metrics.rs`)

```rust
// Before: GPM 후유증 3초 내 미복구 시 데이터 소실
const VRAM_CARRY_FORWARD_TTL: u32 = 3;
const SPARKLINE_CARRY_FORWARD_TTL: u32 = 3;

// After: GPM 상태 복구에 충분한 10초 여유
const VRAM_CARRY_FORWARD_TTL: u32 = 10;
const SPARKLINE_CARRY_FORWARD_TTL: u32 = 10;
```

**변경 2: `memory_total` 무제한 carry-forward (TTL 없음)** (`app.rs`)

```rust
// memory_total은 GPU당 사실상 정적값 — 별도 캐시로 무제한 복원
last_known_vram_total: HashMap<Rc<str>, u64>,

// 매 tick: 유효값 캐시, None 시 캐시에서 복원
if let Some(total) = m.memory_total {
    self.last_known_vram_total.insert(m.uuid.clone(), total);
} else if let Some(&cached) = self.last_known_vram_total.get(&m.uuid) {
    m.memory_total = Some(cached);
}
```

`vram_max`가 `unwrap_or(1)`로 붕괴하지 않아 그래프 스케일 안정.

**변경 3: TTL 만료 후 push(0) — 스파크라인 동결 방지** (`metrics.rs`)

```rust
// Before: TTL 후 push 중단 → 스파크라인 동결 (정체처럼 보임)
// After: TTL 후 0 push → 스파크라인 계속 전진 (하강 = 데이터 소실 시각 표시)
fn push_with_ttl<T: Copy + Default>(...) {
    None => {
        if *none_count <= TTL { /* carry forward */ }
        else if !buf.is_empty() {
            Self::push_ring(buf, T::default(), max); // 0 push
        }
    }
}
```

**변경 4: GPM 샘플 누수 수정** (`nvml.rs`)

```rust
// Before: `?` 연산자로 조기 리턴 시 allocated new_sample 누수
let parent = parent_handle?;   // None이면 new_sample 미해제!
let gi_id = gpu_instance_id?;  // 동일 누수

// After: match로 명시적 free 후 리턴
let (parent, gi_id) = match (parent_handle, gpu_instance_id) {
    (Some(p), Some(g)) => (p, g),
    _ => {
        self.raw_lib.nvmlGpmSampleFree(new_sample);
        return None;
    }
};
```

**변경 5: Drop double-free 수정** (`nvml.rs`)

```rust
// Before: borrow() + iterate → prune_stale_caches에서 이미 free된 포인터 재 free
let prev_map = self.gpm_prev_samples.borrow();
for &sample in prev_map.values() { nvmlGpmSampleFree(sample); }

// After: get_mut() + drain() → 맵 비우면서 1회만 free
for (_, sample) in self.gpm_prev_samples.get_mut().drain() {
    if !sample.is_null() { nvmlGpmSampleFree(sample); }
}
```

**변경 6: O(n²) prev 메트릭 조회 → O(1) HashMap** (`app.rs`)

```rust
// Before: 매 GPU마다 iter().find() 선형 탐색
if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) { ... }

// After: 사전 빌드 HashMap으로 O(1) 인덱스 조회
let prev_by_uuid: HashMap<&Rc<str>, usize> =
    self.metrics.iter().enumerate().map(|(i, m)| (&m.uuid, i)).collect();
if let Some(&idx) = prev_by_uuid.get(&m.uuid) {
    let prev = &self.metrics[idx]; ...
}
```

**변경 7: history entry API + 조건부 HashSet** (`app.rs`)

```rust
// Before: contains_key + insert + get_mut (3회 해시)
if !self.history.contains_key(&m.uuid) {
    self.history.insert(m.uuid.clone(), MetricsHistory::new(MAX_HISTORY));
}
self.history.get_mut(&m.uuid).unwrap().push(m);

// After: entry API (1회 해시)
self.history.entry(m.uuid.clone())
    .or_insert_with(|| MetricsHistory::new(MAX_HISTORY))
    .push(m);

// GPU 제거 시에만 HashSet 빌드 (history.len() > new_metrics.len())
if self.history.len() > new_metrics.len() { /* prune */ }
```

##### 수정 파일

| 파일 | 변경 | 관련 변경 |
|------|------|----------|
| `src/app.rs` | TTL 10 + memory_total 캐시 + O(1) lookup + entry API + 조건부 prune | 변경 1, 2, 6, 7 |
| `src/gpu/metrics.rs` | TTL 10 + push(0) 후 TTL 만료 + `Default` trait bound | 변경 1, 3 |
| `src/gpu/nvml.rs` | GPM 샘플 누수 수정 + Drop drain 패턴 | 변경 4, 5 |

##### 교차 검증 매트릭스

| 검증 항목 | 방법 | 기대 결과 |
|----------|------|----------|
| VRAM 10초 이상 추적 | MIG 환경 장시간 실행 | GPM 후유증 10초 carry-forward → 대부분 복구 |
| vram_max 스케일 안정 | memory_info 실패 시나리오 | memory_total 캐시 → unwrap_or(1) 도달 불가 |
| 스파크라인 계속 전진 | TTL 만료 후 확인 | 0 push로 하강 표시, 동결 없음 |
| GPM 샘플 누수 없음 | MIG + parent_handle=None 시나리오 | 조기 리턴 전 free 확인 |
| Drop double-free 없음 | 종료 시 ASAN/valgrind | drain()으로 1회만 free |
| O(1) lookup 성능 | GPU 8개 이상 환경 | iter().find() 대비 선형 스캔 제거 |
| cargo clippy | 경고 확인 | 신규 경고 없음 ✓ |

##### 자원 누수 감사 결과

| 자원 | v0.3.10 상태 | v0.3.11 수정 |
|------|-------------|-------------|
| GPM 샘플 (MIG `?` 리턴) | **누수** — 할당 후 free 없이 리턴 | ✓ match 패턴으로 명시적 free |
| GPM 샘플 (Drop) | **double-free 위험** — prune 후 재 free | ✓ drain()으로 맵 비우며 1회 free |
| memory_total 캐시 | 없음 — None 시 vram_max=1 | ✓ last_known_vram_total 무제한 캐시 |
| prev 메트릭 조회 | O(n²) 선형 스캔 | ✓ HashMap O(1) |
| history HashMap | 3회 해시 per GPU | ✓ entry API 1회 해시 |
| UUID HashSet | 매 tick 빌드 | ✓ stale 엔트리 존재 시에만 빌드 |

##### v0.3.10 → v0.3.11 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.10: Phase 1.5 GPM 안전 복원 | Phase 1 이후 GPM 호출로 VRAM 보호 | `nvml.rs` MIG 수집 |
| **v0.3.11: VRAM TTL 10초 + memory_total 캐시** | **GPM 후유증 장기 실패 시에도 추적 유지 + 스케일 안정** | `app.rs` carry-forward |
| **v0.3.11: 스파크라인 push(0)** | **TTL 만료 후 그래프 동결 대신 하강 표시** | `metrics.rs` push_with_ttl |
| **v0.3.11: GPM 샘플 안전성** | **메모리 누수 + double-free 제거** | `nvml.rs` get_dram_bw_util_gpm + Drop |
| **v0.3.11: O(1) 조회 + entry API** | **GPU 수 증가 시 tick 오버헤드 선형 유지** | `app.rs` update_metrics |

v0.3.10은 "Mem Ctrl. GPM 안전 복원", v0.3.11은 "VRAM 추적 안정성 + GPM 자원 안전성 + 장기 운영 최적화".

---

#### v0.3.12: GPM 오염 자동 감지 + VRAM 모니터링 안정성 확보

##### 문제

`nvmlGpmMigSampleGet()` 호출이 NVML 드라이버 상태를 cross-tick 오염시켜 `memory_info()`가 영구 실패하는 현상. Phase 1.5 GPM → 다음 tick Phase 1 memory_info 실패 → post-probe 대상 없음(`probe_idx=None`) → GPM 계속 실행 → 재오염 무한 루프.

##### 수정 내용 (3개 변경)

**변경 1: GPM post-probe 오염 감지** (`nvml.rs`)

Phase 1.5 GPM 호출 후 Phase 1에서 memory_info 성공했던 MIG 디바이스로 재검증. 실패 시 → `gpm_disabled_parents` HashSet에 parent 등록, 이후 tick GPM 영구 비활성화.

**변경 2: `gpm_disabled_parents` 자동 정리** (`nvml.rs`)

`prune_stale_caches`에서 제거된 parent GPU의 disabled 플래그 자동 정리 → GPU hot-remove/MIG 재구성 시 stale 엔트리 방지.

**변경 3: CLAUDE.md GPM 오염 감지 설계 기록** (`CLAUDE.md`)

GPM 호출 후 post-probe → 오염 감지 → 영구 비활성화 설계 의도를 Key Design Decisions에 추가.

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/gpu/nvml.rs` | `gpm_disabled_parents` RefCell<HashSet> 추가, Phase 1.5 post-probe 오염 감지 + GPM 영구 비활성화, `prune_stale_caches` disabled 정리 |
| `CLAUDE.md` | GPM 오염 감지 설계 문서화 |

---

#### v0.3.13: GPM 오염 무한루프 차단 + 장기 운영 미세 최적화

##### 문제

v0.3.12의 post-probe는 **같은 tick 내 즉시 발현되는 오염**만 감지. GPM 오염이 지연 발현(다음 tick에서야 memory_info 실패)되면:

1. Tick N: Phase 1 memory_info 성공 → GPM 실행 → post-probe 통과 (오염 미발현)
2. Tick N+1: 전체 MIG memory_info 실패 → `probe_idx=None` → post-probe 스킵 → GPM 재실행 → 재오염
3. 매 tick 반복 → carry-forward TTL 10초 후 VRAM N/A

##### 수정 내용 (3개 변경)

**변경 1: `probe_idx=None`일 때 GPM 스킵** (`nvml.rs`)

전체 MIG memory_info 실패 시 GPM 자체를 스킵하여 오염 사이클 차단. NVML이 자체 복구(1~3 tick)되면 probe_idx가 Some으로 돌아오고, 그때 GPM 재시도 + post-probe가 정상 작동.

```rust
// 수정 전: probe_idx=None → GPM 실행 → 재오염 무한루프
let probe_idx = phase1.iter().position(|(_, _, m)| m.memory_used.is_some());
for (mig_handle, gi_id, metrics) in &mut phase1 { ... }  // GPM 무조건 실행
if let Some(idx) = probe_idx { ... }                       // probe 없으면 스킵

// 수정 후: probe_idx=None → GPM 전체 스킵 → NVML 복구 기회 제공
if let Some(pidx) = probe_idx {
    for (mig_handle, gi_id, metrics) in &mut phase1 { ... }  // GPM 실행
    // post-probe (pidx 보장)
    let (probe_handle, _, _) = &phase1[pidx];
    ...
}
// else: skip GPM → break corruption cycle
```

**변경 2: `proc_seen_pids` 재활용으로 매 tick HashSet 할당 제거** (`nvml.rs`)

`proc_name_cache` 정리 시 새 `HashSet<u32>` 할당 대신, collect phase 종료 후 유휴 상태인 `proc_seen_pids` RefCell 재활용.

**변경 3: `none_count` TTL 상한 적용** (`metrics.rs`)

`push_with_ttl`에서 TTL 초과 후에도 `none_count`가 무한 증가하던 문제 수정. `TTL+1`에서 cap하여 영구 N/A 메트릭에서 의미 없는 증분 방지.

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/gpu/nvml.rs` | Phase 1.5 `probe_idx=None` 시 GPM 스킵 + `proc_seen_pids` 재활용 |
| `src/gpu/metrics.rs` | `none_count` TTL+1 cap |

##### 검증 매트릭스

| 검증 항목 | 방법 | 기대 결과 |
|----------|------|----------|
| GPM 지연 오염 시 VRAM 복구 | MIG 장시간 실행, GPM 오염 지연 발현 시나리오 | probe_idx=None → GPM 스킵 → 1~3 tick 후 NVML 복구 → VRAM 정상 |
| proc_name_cache 정리 할당 | tick당 힙 할당 추적 | 새 HashSet 할당 0회 (proc_seen_pids 재활용) |
| none_count 오버플로우 방지 | 영구 N/A 메트릭 장시간 | none_count ≤ TTL+1 (11) 고정 |
| cargo clippy | 경고 확인 | 신규 경고 없음 ✓ |

##### 자원 누수 감사 결과

| 자원 | v0.3.12 상태 | v0.3.13 수정 |
|------|-------------|-------------|
| GPM 오염 무한루프 | **probe_idx=None 시 GPM 재실행 → 재오염** | ✓ GPM 스킵으로 사이클 차단 |
| proc_name_cache 정리 HashSet | 매 tick 새 HashSet 할당 | ✓ proc_seen_pids 재활용 |
| none_count 무한 증가 | u32 무한 증분 (실질 무해하나 불필요) | ✓ TTL+1에서 cap |

##### v0.3.10 → v0.3.13 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.10: Phase 1.5 GPM 2-phase 분리 | Phase 1 VRAM 수집 후 GPM 호출 → 같은 tick VRAM 보호 | `nvml.rs` MIG 수집 |
| v0.3.11: VRAM TTL 10초 + memory_total 캐시 | GPM 후유증 장기 실패 시에도 추적 유지 + 스케일 안정 | `app.rs` carry-forward |
| v0.3.12: post-probe 오염 감지 + GPM 영구 비활성화 | 같은 tick 즉시 발현 오염 → GPM 차단 | `nvml.rs` Phase 1.5 |
| **v0.3.13: probe_idx=None 시 GPM 스킵** | **지연 발현 오염 (다음 tick) → 재오염 무한루프 차단** | `nvml.rs` Phase 1.5 |
| **v0.3.13: 매 tick 할당 최적화** | **proc_seen_pids 재활용 + none_count cap** | `nvml.rs`, `metrics.rs` |

v0.3.12는 "GPM 오염 자동 감지 + 영구 비활성화", v0.3.13은 "GPM 지연 오염 무한루프 차단 + 장기 운영 미세 최적화".

---

#### v0.3.14: MIG Mem Ctrl 근본 해결 + Startup GPM Probe + 렌더링 최적화

##### 문제

v0.3.13까지의 GPM 방어 계층(post-probe, 무한루프 차단)은 모두 **GPM 오염 사후 대응**. 근본 원인은 GPM 호출 자체 — Mem Ctrl을 GPM 없이 표시할 수 있으면 GPM 호출이 불필요해져 오염이 원천적으로 사라짐.

##### 수정 내용 (3개 변경)

**변경 1: Parent Memory Samples Mem Ctrl 폴백** (`nvml.rs`)

부모 GPU의 `nvmlDeviceGetSamples(MEMORY_UTILIZATION)` raw 값(/10000)을 MIG 슬라이스 비율로 스케일링 → GPM 없이 Mem Ctrl 표시. Phase 1에서 `memory_util`을 채우므로 Phase 1.5 GPM이 자연스럽게 억제됨.

**변경 2: Startup GPM Safety Probe** (`nvml.rs`)

`NvmlCollector::new()`에서 MIG-enabled 부모 GPU마다 GPM 안전성 1회 검증. `nvmlGpmMigSampleGet()` 호출 후 `memory_info()` 재확인 → 실패 시 `gpm_disabled_parents`에 즉시 등록, 런타임 오염 원천 차단.

**변경 3: 렌더링 최적화** (`dashboard.rs`)

`selected_metrics()` 7회 중복 호출 → 1회 캐싱, Cow::Borrowed 정적 타이틀, Vec::with_capacity Span 벡터.

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/gpu/nvml.rs` | `get_mem_util_from_samples()` 추가 + Phase 1 Mem Ctrl 폴백 + Startup GPM Probe |
| `src/ui/dashboard.rs` | 렌더링 중복 제거 + 정적 타이틀 + Vec 사전 할당 |

---

#### v0.3.15: Mem Ctrl 다중 폴백 + GPM 방어적 차단 강화

##### 문제

v0.3.14의 Mem Ctrl 근본 해결은 `nvmlDeviceGetSamples(type=1 MEM_UTIL)`이 항상 작동한다고 가정. 그러나 일부 드라이버/아키텍처에서 이 sample type을 지원하지 않아 `parent_sampled_mem_util = None` → GPM 억제 체인이 깨짐:

```
nvmlDeviceGetSamples(type=1) 실패 → parent_sampled_mem_util = None
  → Phase 1 Mem Ctrl 폴백 스킵 → memory_util = None (Mem Ctrl N/A)
    → Phase 1.5 GPM 억제 실패 → GPM 실행 → NVML 오염
      → memory_info() 연쇄 실패 → 10 tick 후 VRAM N/A
```

##### 근본 원인

두 증상(Mem Ctrl 처음부터 N/A, VRAM 10초 후 N/A)은 하나의 인과 체인:
1. `nvmlDeviceGetSamples(type=1)` 미지원 → Mem Ctrl에 값 없음
2. `memory_util`이 None → Phase 1.5 GPM 억제 실패
3. GPM이 NVML 상태 오염 → `memory_info()` 실패 → carry-forward TTL 만료 → VRAM N/A

##### 수정 내용 (4개 변경)

**변경 1: Parent `utilization_rates().memory` 폴백 추가** (`nvml.rs`)

Phase 1 루프 전에 parent의 표준 `utilization_rates()` API로 memory util 조회. `nvmlDeviceGetSamples(type=1)`보다 드라이버 지원 범위가 넓음.

```rust
let parent_util_rates_mem: Option<u32> =
    parent_device.utilization_rates().ok().map(|u| u.memory);
```

**변경 2: Mem Ctrl 이중 폴백 분기** (`nvml.rs`)

`parent_sampled_mem_util`이 None일 때 `parent_util_rates_mem`으로 스케일링. memory_util이 채워지면 → Phase 1.5 GPM 자동 억제.

```rust
if metrics.memory_util.is_none() {
    if let Some(p_mem_util) = parent_sampled_mem_util {
        // (a) raw samples 스케일링 (기존)
    } else if let Some(p_rates_mem) = parent_util_rates_mem {
        // (b) utilization_rates().memory 스케일링 (NEW)
    }
}
```

**변경 3: GPM 방어적 차단 조건** (`nvml.rs`)

`parent_sampled_mem_util`이 None이면 드라이버 확장 API가 불안정하다는 신호 → GPM 진입 자체를 차단 (defense-in-depth).

```rust
let parent_mem_samples_ok = parent_sampled_mem_util.is_some();
if parent_info.gpm_supported && !gpm_disabled && parent_mem_samples_ok {
```

**변경 4: Startup GPM Probe 2회 반복** (`nvml.rs`)

일부 드라이버는 첫 GPM 호출에서는 오염 없이 2회째부터 오염 발생. 2회 프로빙으로 지연 오염 감지.

##### 수정 파일 목록

| 파일 | 변경 내용 |
|------|-----------|
| `src/gpu/nvml.rs` | `parent_util_rates_mem` 추가 + Mem Ctrl `else if` 폴백 + GPM `parent_mem_samples_ok` 가드 + Startup Probe 2회 |
| `CLAUDE.md` | 6단계 폴백 체인 문서 업데이트 |

##### 검증 매트릭스

| 검증 항목 | 방법 | 기대 결과 |
|----------|------|----------|
| Mem Ctrl 표시 (samples 미지원 드라이버) | MIG 환경에서 실행 | `utilization_rates().memory` 폴백으로 Mem Ctrl 값 표시 |
| VRAM 장기 안정성 | 10분+ 연속 실행 | VRAM N/A 전환 없음 (GPM 미실행) |
| GPM 억제 확인 | `parent_sampled_mem_util=None` 시 | Phase 1.5 GPM 진입 차단 |
| Startup Probe 지연 오염 감지 | 2회째 GPM에서 오염 발생 드라이버 | 2회 프로빙으로 감지 → `gpm_disabled_parents` 등록 |
| cargo clippy + fmt | 정적 분석 | 신규 경고 없음 ✓ |

##### 자원 누수 감사 결과 (전체 코드베이스)

| 자원 | 상태 | 상한 |
|------|------|------|
| `proc_name_cache` | ✓ 매 tick active PID만 retain + shrink | GPU수 × 5 |
| `device_cache` | ✓ `prune_stale_caches()` 매 tick | active GPU수 |
| `gpm_prev_samples` | ✓ retain + free + Drop drain | active MIG수 |
| VecDeque 히스토리 | ✓ 고정 300 ring buffer | 300 × 9 메트릭 |
| `sample_buf` | ✓ grow-only + >8x시 shrink | ~2KB |
| `vram_fail_count` | ✓ GPU 제거 시 retain | active GPU수 |
| Per-frame format!() | ~46회/frame, GPU 수집(100-500ms) 대비 < 0.1% | 최적화 불필요 |

##### v0.3.10 → v0.3.15 방어 계층 관계

| 방어 계층 | 보호 범위 | 적용 시점 |
|----------|----------|----------|
| v0.3.10: Phase 1.5 GPM 2-phase 분리 | Phase 1 VRAM 수집 후 GPM 호출 → 같은 tick VRAM 보호 | `nvml.rs` MIG 수집 |
| v0.3.11: VRAM TTL 10초 + memory_total 캐시 | GPM 후유증 장기 실패 시에도 추적 유지 + 스케일 안정 | `app.rs` carry-forward |
| v0.3.12: post-probe 오염 감지 + GPM 영구 비활성화 | 같은 tick 즉시 발현 오염 → GPM 차단 | `nvml.rs` Phase 1.5 |
| v0.3.13: probe_idx=None 시 GPM 스킵 | 지연 발현 오염 → 재오염 무한루프 차단 | `nvml.rs` Phase 1.5 |
| v0.3.14: Parent Memory Samples + Startup Probe | GPM 없이 Mem Ctrl 표시 → GPM 호출 자연 억제 + 시작 시 오염 감지 | `nvml.rs` Phase 1, `new()` |
| **v0.3.15: utilization_rates() 폴백 + GPM 방어적 차단** | **samples 미지원 시 대체 소스 + GPM 진입 자체 차단 (defense-in-depth)** | `nvml.rs` Phase 1, Phase 1.5 |
| **v0.3.15: Startup Probe 2회 반복** | **지연 오염 드라이버에서 시작 시 감지 강화** | `nvml.rs` `new()` |

v0.3.14는 "GPM 없는 Mem Ctrl 근본 해결", v0.3.15는 "samples 미지원 드라이버 대응 + GPM 방어적 차단 강화".

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
│   ├── running_compute/graphics_procs ~0.5ms  compute+graphics 통합 + top-5만 /proc 이름 읽기
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
│   └── SystemHistory                     ~7 KB    (4 VecDeque × 300 × 4-8B, ram_used_pct/ram_cached_pct 포함)
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
| NVML 샘플 버퍼 shrink | `nvml.rs` | grow-only 버퍼 무한 증가 가능 → capacity > needed×2 시 `shrink_to(needed×2)` 자동 축소 |
| Sparkline RightToLeft 방향 통일 | `dashboard.rs` | 5개 Sparkline 모두 `RenderDirection::RightToLeft` 적용 → RAM 세그먼트 차트와 동일한 우측→좌측 진행 방향 |
| RAM 차트 zero-alloc 렌더링 | `dashboard.rs` | 매 프레임 `Vec<ColSegment>` 할당 → 직접 iterator + buffer write (할당 0) |
| RAM 세그먼트 차트 적층 보정 | `dashboard.rs` | used fractional 행이 cached 시작점을 밀어 cached가 매 컬럼 ~1행 손실 → `cached_base` 도입으로 used 소수점 행 유무에 따라 cached 기준점 정확히 산정 |
| RAM 차트 fractional 셀 bg 색상 | `dashboard.rs` | used fractional 문자(`▃▄` 등)의 빈 상단이 배경색으로 남아 cached와 시각적 갭 → `cell.set_bg(Color::Blue)` 적용, 셀 내 하단=used(fg) 상단=cached(bg) 밀착 표시 |
| RAM 계산 정확도 수정 | `dashboard.rs` | `used = ram_used - (avail-free)` (이중 차감) → `used = total - available` (정확한 비해제 가능 메모리) |
| `format_pstate` 제로 할당 | `nvml.rs` | 매 tick `"P0".to_string()` String 할당 → `&'static str` 반환 (할당 0) |
| `format_architecture` 제로 할당 | `nvml.rs` | 동일 패턴: `"Ampere".to_string()` → `&'static str` |
| `format_throttle_reasons` Vec 제거 | `nvml.rs` | `Vec::new()` + `push` + `join()` → macro로 `String`에 직접 append (Vec 할당 제거) |
| `GIB_F64` 모듈 상수 | `metrics.rs` | `1024.0 * 1024.0 * 1024.0` 중복 계산 → `const GIB_F64` 1회 정의, 전역 재사용 |
| `ram_breakdown()` 계산 통합 | `metrics.rs` | `draw_ram_swap` + `draw_memory_legend`에서 RAM 분해 중복 계산 → `SystemMetrics::ram_breakdown()` 1회 호출 |
| `truncate_str()` 제로 할당 | `dashboard.rs` | `proc.name.chars().take(15).collect::<String>()` 프레임당 5개 할당 → `&str` 슬라이싱 (할당 0) |
| `Rc<str>` 문자열 공유 | `nvml.rs`, `metrics.rs`, `app.rs` | `DeviceInfo`/`GpuMetrics`의 name·uuid·compute_capability를 `Rc<str>`로 변경 → clone 시 heap 할당 제거 (포인터 카운트 증가만) |
| `ram_breakdown()` 1회 호출 | `dashboard.rs` | `draw_ram_bars` + `draw_memory_legend`에서 중복 계산 → `draw_system_charts`에서 1회 계산 후 두 함수에 전달 |
| 프로세스명 캐싱 | `nvml.rs` | 매 tick `/proc/{pid}/comm` I/O → `HashMap<u32, String>` 캐시 + tick당 dead PID 자동 정리 |
| NVML 버퍼 shrink 임계값 강화 | `nvml.rs` | `capacity > needed*2` → `capacity > floor*8` 기준으로 변경, 프로세스/샘플 수 변동 시 shrink↔resize 쓰레싱 방지 |
| `device_cache` HashMap 방어적 shrink | `nvml.rs` | MIG 재설정 반복 시 HashMap capacity 무한 증가 → `capacity > len*4` 시 자동 축소 |
| Memory 패널 우측 통합 | `dashboard.rs` | 좌측 Memory 박스 제거 → 우측 System Charts에 RAM/SWP 바 통합, CPU 코어 표시 영역 확장 |
| `active_handles` Vec 재사용 | `nvml.rs` | 매 tick `Vec::with_capacity(N)` 할당/해제 → `RefCell<Vec<usize>>` 필드로 재사용 (tick당 할당 0) |
| Sparkline 타이틀 `Cow<str>` | `dashboard.rs` | 정적 문자열("N/A", 폴백) `to_string()` 할당 → `Cow::Borrowed` 제로 할당, 동적 값만 `format!` |
| proc_name_cache HashSet 정리 | `nvml.rs` | O(n·m) 중첩 순회(`retain` 내 `any` × `any`) → HashSet 기반 O(n+m) 조회 (프로세스 많은 시스템에서 CPU 절감) |
| Top Processes 헤더 정적 `&str` | `dashboard.rs` | 매 프레임 `format!()` 3회 호출 → 정적 문자열 `&str` Span으로 변경 (프레임당 3회 String 할당 제거) |
| Top Processes 컬럼 정렬 수정 | `dashboard.rs` | 헤더(하드코딩 8+22+4) ↔ 데이터({:<7}+{:<15}+{:>10}) 폭 불일치 → 동일 포맷 폭으로 통일 |
| Compute + Graphics 프로세스 통합 | `nvml.rs` | compute만 수집 → compute + graphics 양쪽 수집 + HashSet PID dedup, VRAM Unavailable 프로세스 보존 |
| `GpuProcessInfo.vram_used` Option화 | `metrics.rs` | `u64` → `Option<u64>`, VRAM Unavailable 프로세스 "N/A" 표시 (기존: 필터링 제거) |
| MIG 프로세스 부모 디바이스 폴백 | `nvml.rs` | MIG 핸들 프로세스 API 실패 시 부모 GPU에서 프로세스 쿼리 → `gpu_instance_id` 필터링으로 MIG 인스턴스별 분배 (Phase 3) |
| Top Processes carry-forward | `app.rs` | NVML 프로세스 API 간헐적 실패 시 이전 틱 프로세스 목록 유지 → 깜빡임 방지 |
| Phase 3 gi_id 폴백 완화 | `nvml.rs` | 부모 GPU 프로세스에 `gpu_instance_id` 없을 때 전체 프로세스 표시 (기존: 전부 필터링) |
| datetime 포맷 캐시 | `dashboard.rs` | 매 tick `chrono::format().to_string()` → `thread_local!` 캐시, 초 단위 변경 시에만 재생성 |
| GPU history HashMap 정확한 프루닝 | `app.rs` | `len > metrics.len()` 조건 제거 → UUID 불일치 시 항상 프루닝 + `shrink_to()` 추가 (MIG 재구성 시 orphan 방지) |
| proc_name_cache shrink 임계값 개선 | `nvml.rs` | `len.max(16) * 4` → `target * 2` 기준으로 변경, 대량 PID 소멸 시 더 적극적 메모리 회수 |
| Sparkline 데이터 방향 수정 | `dashboard.rs` | `data[0]=oldest`가 오른쪽 끝에 표시되는 버그 → `.rev()`로 `data[0]=newest` 변환, 최신 데이터가 오른쪽 끝에 정확히 표시 |
| f32→u64 rounding | `dashboard.rs` | `v as u64` truncation (99.7→99) → `v.round() as u64` (99.7→100) CPU sparkline 정밀도 개선 |
| `GpuProcessInfo::name` → `Rc<str>` | `metrics.rs`, `nvml.rs` | 매 tick `String::clone()` 힙 복사 → `Rc::clone()` 포인터 카운트만 (GPU당 프로세스 5개 × tick 힙 할당 제거) |
| `throttle_reasons` → `Cow<'static, str>` | `metrics.rs`, `nvml.rs` | 매 tick `String` 힙 할당 → "None", "Idle" 등 단일 플래그는 `Cow::Borrowed` 제로 할당 (실 사용의 90%+ 커버) |
| `proc_name_cache` → `HashMap<u32, Rc<str>>` | `nvml.rs` | 캐시 히트 시 `String::clone()` → `Rc::clone()` 포인터 카운트만, 프로세스명 공유 비용 제거 |
| PCIe sparkline 타이틀 명확화 | `dashboard.rs` | 타이틀 "TX/RX" 표기가 그래프 내용(TX만)과 불일치 → "PCIe TX:N / RX:N MB/s"로 명확화 |
| Sparkline carry-forward TTL | `metrics.rs` | 메트릭 None 시 마지막 값 무한 반복 → `none_counts[9]` 배열로 메트릭별 3틱 TTL 적용, 초과 시 push 중단 (스파크라인 정체 방지) |
| `get_process_utilization` Option 반환 | `nvml.rs` | API 실패 시 `(0, 0)` 반환 → `Option<(u32, u32)>` 반환, idle 0%와 실패를 구분하여 잘못된 폴백 스케일링 방지 |
| `collect_all()` 디바이스별 에러 격리 | `nvml.rs` | `device_by_index(i)?`로 1개 GPU 에러 시 전체 수집 실패 → `match ... continue`로 해당 GPU만 건너뛰기 |
| MIG display name 캐싱 | `nvml.rs` | 매 tick `format!().into()` 새 `Rc<str>` → `DeviceInfo.mig_display_name` 캐시, `Rc::clone()` 재사용 |
| PID dedup HashSet 재사용 | `nvml.rs` | 매 tick `HashSet::new()` per-device → `proc_seen_pids: RefCell<HashSet<u32>>` 재사용 (tick당 할당 0) |
| `make_bar()` 룩업 테이블 | `dashboard.rs` | 코어당 `String` 할당 → `BAR_TABLE` thread-local 룩업, `&str` 참조만 (128코어: 128 alloc → 0/draw) |
| 시간 문자열 zero-alloc | `dashboard.rs` | `clone()` + `format!()` 2 alloc/draw → `write!` 버퍼 재사용 + `as_str()` 참조 (0 alloc/draw) |
| `phase1` Vec 사전 할당 | `nvml.rs` | `Vec::new()` → `Vec::with_capacity(max_count)`, MIG 수집 시 realloc 제거 |
| entries/parent_procs 사전 할당 | `nvml.rs` | `Vec::new()` → `Vec::with_capacity(16)`, 프로세스 수집 시 첫 push 시 alloc 제거 |

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
| 디바이스 캐시 tick별 정리 | `nvml.rs` | active handle 추적 → stale `DeviceInfo` 제거, MIG 재구성 시 리소스 누수 방지 |
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
| GPU 히스토리 자동 정리 + shrink | `app.rs` | MIG 재구성/GPU 제거 시 UUID 불일치 감지 → orphan 엔트리 자동 삭제 + capacity 축소 |
| NVML 샘플 버퍼 shrink-to-fit | `nvml.rs` | capacity > floor×8 시 자동 축소, 프로세스/샘플 수 변동 시 shrink↔resize 쓰레싱 방지 |
| DeviceInfo 캐시 1회 수집 + `Rc<str>` | `nvml.rs` | 정적 정보(arch, CC 등)는 첫 호출 시 캐시, clone 시 포인터 카운트만 증가 (heap 할당 0) |
| 프로세스명 캐싱 + dead PID 정리 | `nvml.rs` | `/proc/{pid}/comm` I/O 캐싱 (`HashMap<u32, Rc<str>>`), 매 tick 현재 top-5에 없는 PID 자동 제거, 캐시 히트 시 `Rc::clone()`만 (힙 할당 0) |
| `throttle_reasons` `Cow<'static, str>` | `nvml.rs` | "None", "Idle" 등 빈번한 단일 플래그는 `Cow::Borrowed` 제로 할당, 복합 플래그만 `Cow::Owned` |
| Top Processes PID 생존 확인 carry-forward | `app.rs` | NVML 프로세스 API 간헐적 실패 시 `/proc/{pid}` 존재 확인 후 살아있는 프로세스만 유지, 종료된 프로세스는 즉시 제거 |
| VRAM carry-forward TTL (10회) | `app.rs` | `memory_info()` 연속 실패 10회까지 이전 값 유지 (memory_info 연속 실패 복구 여유), 초과 시 None("N/A") 전환 → 오래된 높은 값 무기한 표시 방지 |
| `memory_total` 무제한 캐시 | `app.rs` | GPU당 정적값인 memory_total을 `last_known_vram_total`에 무제한 보존 → memory_info() 실패 시에도 vram_max 스케일 안정 |
| 스파크라인 TTL 후 push(0) | `metrics.rs` | TTL 만료 후 push 중단 대신 0 push → 그래프 동결 방지, 하강으로 데이터 소실 시각 표시 |
| prev 메트릭 O(1) HashMap 조회 | `app.rs` | iter().find() O(n²) → HashMap 인덱스 O(1), GPU 8+ 환경에서 tick 오버헤드 감소 |
| history entry API | `app.rs` | contains_key+insert+get_mut 3회 해시 → entry() 1회 해시로 통합 |
| `/proc/{pid}` 경로 버퍼 재사용 | `app.rs` | `proc_path_buf: String` 재사용으로 매 PID마다 format! 힙 할당 제거, 300시간 기준 ~27억 회 할당 방지 |
| datetime 포맷 캐시 | `dashboard.rs` | `thread_local!` 캐시로 초 단위 변경 시에만 `chrono::format()` 호출 (틱당 String 할당 1회 절감) |
| device_cache 방어적 shrink | `nvml.rs` | MIG 재설정 반복 시 HashMap capacity 무한 증가 방지 → `capacity > len*4` 시 자동 축소 |
| proc_name_cache HashSet 기반 정리 | `nvml.rs` | 매 tick dead PID 정리 시 O(n·m) 중첩 순회 → `HashSet<u32>` 기반 O(n+m) 조회로 변경, 프로세스 수 증가 시에도 일정 성능 |
| sysinfo targeted refresh | `main.rs` | `refresh_cpu_usage()` + `refresh_memory()`만 호출, 프로세스 누적 없음 |
| `active_handles` HashSet 재사용 | `nvml.rs` | `RefCell<HashSet<usize>>` 재사용 + O(1) contains 조회, prune_stale_caches O(n²)→O(n) |
| history/vram_fail_count UUID HashSet 정리 | `app.rs` | GPU 제거 감지 시 이중 `.any()` O(n×m) → HashSet O(n) 단일 순회 |
| Sparkline carry-forward TTL | `metrics.rs` | 메트릭 None 시 마지막 값 무한 반복으로 스파크라인 정체 → `none_counts[9]` 배열로 메트릭별 10틱 제한, 초과 시 0 push (동결 대신 하강 표시) |
| `get_process_utilization` 실패/idle 구분 | `nvml.rs` | API 실패 `(0, 0)` = idle 0% 구분 불가 → `Option<(u32, u32)>` 반환, idle 0%는 정상 보고·실패는 다음 폴백 진행 |
| `collect_all()` 디바이스별 에러 격리 | `nvml.rs` | 1개 GPU `device_by_index` 에러 시 전체 메트릭 수집 중단 → 해당 GPU만 skip, 나머지 GPU 정상 수집 유지 |
| MIG display name 캐싱 | `nvml.rs` | MIG 이름 `Rc<str>` 첫 tick만 생성, 이후 `Rc::clone()` 포인터 카운트만 (MIG 인스턴스당 tick 힙 할당 제거) |
| PID dedup HashSet 재사용 | `nvml.rs` | `RefCell<HashSet<u32>>` 필드 재사용, 첫 tick 이후 할당 0 (clear만) |
| BAR_TABLE 룩업 테이블 | `dashboard.rs` | `thread_local!` bar 문자열 테이블, 터미널 리사이즈 시에만 재빌드, 128코어에서도 draw당 bar 할당 0 |
| RAM 차트 fractional 셀 bg 색상 | `dashboard.rs` | used fractional 문자의 빈 상단 → `cell.set_bg(Color::Blue)` 적용, used-cached 경계 시각적 밀착 보장 |
| proc_name_cache 정리 시 HashSet 재활용 | `nvml.rs` | `proc_seen_pids` RefCell 재활용으로 매 tick 새 HashSet 할당 제거 |
| none_count TTL 상한 적용 | `metrics.rs` | `push_with_ttl` none_count를 TTL+1에서 cap → 영구 N/A 메트릭에서 의미 없는 u32 증분 방지 |
| Parent Memory Samples Mem Ctrl 폴백 | `nvml.rs` | `nvmlDeviceGetSamples(MEM_UTIL)` → 부모 GPU 메모리 컨트롤러 util을 MIG 슬라이스 비율로 스케일링, Ampere/Hopper 모두 Mem Ctrl 표시 가능 |
| Parent `utilization_rates().memory` 폴백 | `nvml.rs` | `nvmlDeviceGetSamples(type=1)` 미지원 드라이버에서 parent의 표준 `utilization_rates().memory` API로 Mem Ctrl 대체 소스 제공 |
| `--debug` 진단 로깅 | `main.rs`, `nvml.rs` | `--debug` 플래그로 모든 NVML API 호출 결과를 `/tmp/mig-gpu-mon-debug.log`에 기록, 장기 운영 중 문제 진단 가능 |
| `selected_metrics()` 캐싱 | `dashboard.rs` | `draw_gpu_charts()`에서 7회 중복 호출 → 1회 캐싱, 프레임당 HashMap lookup 6회 제거 |
| Cow::Borrowed 정적 타이틀 | `dashboard.rs` | 차트 타이틀 리터럴에 `.into()` (Cow::Owned) → `Cow::Borrowed` 직접 사용, 프레임당 불필요한 String 힙 할당 6~8개 제거 |
| Vec::with_capacity Span 벡터 | `dashboard.rs` | `Vec::new()` → `Vec::with_capacity(4~6)`, 첫 push 시 realloc 3회 제거 |

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
| Parent Memory Samples 폴백 | `nvml.rs` | `nvmlDeviceGetSamples(MEM_UTIL)` → MIG 슬라이스 스케일링으로 Mem Ctrl 표시 |
| Parent `utilization_rates().memory` 폴백 | `nvml.rs` | samples 미지원 드라이버에서 표준 API로 Mem Ctrl 대체 소스 제공 |
| `--debug` 진단 모드 | `main.rs`, `nvml.rs` | `--debug` 플래그로 NVML API 호출별 ret 코드·에러 상세를 파일 로깅 → 장기 운영 중 문제 원인 추적 |

## Why Rust

- **NVML FFI 직접 호출** — MIG 제한 우회를 위한 raw C API 접근 가능
- **제로 오버헤드** — 모니터링 도구 자체의 CPU/메모리 사용 극소화, GPU 워크로드에 영향 없음
- **단일 바이너리** — 클라우드/컨테이너 환경에서 `scp` 또는 `COPY`만으로 배포

## License

MIT
