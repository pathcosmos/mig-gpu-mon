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
├─ Memory ────────────────────────┤ UUID: MIG-a1b2...  Arch:Ampere CC:8.0│ │
│ RAM ▮▮▮▮▮▯▯ 89.2/256.0 GiB … │ VRAM 12288 MB / 20480 MB (60.0%)    │ │
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
│   │   │   └── RAM / Swap        Length(4)  " Memory "
│   │   │       ├── RAM line                 "RAM ▮▮▯▯ {used}/{total} GiB ({pct}%)"
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
| RAM 바 | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
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

- MIG 인스턴스별 GPU Util, Mem Ctrl(메모리 컨트롤러), SM Util, VRAM 사용량 실시간 표시
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
- 시스템 RAM / Swap 사용량
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
    1. GPU 메트릭 수집 (NVML API)
       - 물리 GPU: utilization_rates(), memory_info(), temperature(), ...
       - MIG 인스턴스: utilization_rates() 실패 시
         → nvmlDeviceGetProcessUtilization() 폴백으로 SM/Mem util 집계
    2. 시스템 메트릭 수집 (sysinfo)
       - CPU 코어별 usage, 총 RAM/Swap
    3. TUI 렌더링 (ratatui)
    4. 이벤트 대기 (crossterm poll, interval 만큼 블로킹)
       - 키 입력 처리 또는 tick → 다음 루프
}
```

## MIG Utilization 수집 메커니즘

### 3단계 폴백 아키텍처

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

#### 메모리 컨트롤러 Utilization

드라이버 535.x MIG에서 메모리 컨트롤러 utilization의 폴백 경로는 존재하지 않는다. 오해를 유발하는 0% 대신 "N/A"로 표시한다.

> **참고:** NVIDIA 드라이버 550+ (CUDA 12.4+)부터 MIG 디바이스 핸들에서 `nvmlDeviceGetUtilizationRates()` 정식 지원이 추가되어, 3단계 폴백이 불필요해진다.

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
| **디스크 I/O** | **0** | 파일 읽기/쓰기 없음 |
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
│   ├── running_compute_procs  ~0.5ms
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
| CPU 버퍼 재사용 | `main.rs` | 매 tick `Vec::new()` → `cpu_buf.clear()` + extend (용량 유지) |
| sparkline 변환 버퍼 | `dashboard.rs` | draw마다 5개 `Vec<u64>` 할당 → `thread_local!` scratch 1개 재사용 |
| 프로세스 partial sort | `nvml.rs` | O(n log n) 전체 정렬 → O(n) `select_nth_unstable_by` (프로세스 > 5일 때) |
| CPU cores Vec 재사용 | `dashboard.rs` | 매 draw마다 Vec 할당 → `thread_local!` 버퍼 재사용 |
| `make_bar()` 문자열 | `dashboard.rs` | `.repeat()` 2회 연결 → `String::with_capacity` + push loop |
| HashMap uuid clone | `app.rs` | 매 tick `uuid.clone()` → `contains_key` 후 miss 시에만 clone |

### 최적화 상세: CPU (시스템 호출 최소화)

| 최적화 | 위치 | 효과 |
|--------|------|------|
| `System::new()` | `main.rs` | `new_all()` 대비 프로세스/디스크/네트워크 전체 스캔 제거 |
| 타겟 refresh | `main.rs` | `refresh_cpu_usage()` + `refresh_memory()`만 — /proc/stat, /proc/meminfo 2개만 읽기 |
| 기본 interval 1000ms | `main.rs` | 500ms 대비 모든 시스콜+NVML 호출 횟수 절반 |
| CPU priming | `main.rs` | sysinfo 첫 `refresh_cpu_usage()` 0% 반환 방지 — 초기화 시 1회 선호출 |

### 최적화 상세: GPU (NVML 호출 최소화)

| 최적화 | 위치 | 효과 |
|--------|------|------|
| `utilization_rates()` 우선 | `nvml.rs` | MIG에서도 일단 시도, 실패 시에만 process util 폴백 (추가 IPC 2회 절약) |
| `nvmlDeviceGetSamples` 폴백 | `nvml.rs` | `utilization_rates()` MIG 실패 시 부모 GPU 레벨 샘플링 → 슬라이스 비율 스케일링, 버퍼 `RefCell<Vec>` 재사용 |
| process util 2-pass | `nvml.rs` | 1차 count=0으로 크기 확인, 2차 데이터 수집 — 과다 버퍼 할당 방지 |
| `RefCell` 내부 가변성 | `nvml.rs` | `&self`로 NVML 핸들 빌린 상태에서 캐시/버퍼 수정 가능, borrow checker 충돌 없이 |
| GPU 자원 0 사용 | 설계 | NVML은 읽기 전용 드라이버 쿼리 — CUDA 컨텍스트 없음, VRAM 할당 없음 |

### 최적화 상세: Binary Size

| 설정 | 값 | 효과 |
|------|-----|------|
| `opt-level` | 3 | 최대 최적화 |
| `lto` | true | Link-Time Optimization, 미사용 코드 제거 |
| `strip` | true | 디버그 심볼 완전 제거 |
| `tokio` 제거 | — | async 미사용, 동기 이벤트 루프로 충분 — 바이너리 ~200KB 절약 |
| 최종 크기 | **~1.5MB** | 단일 바이너리 (libc 동적 링크) |

## Why Rust

- **NVML FFI 직접 호출** — MIG 제한 우회를 위한 raw C API 접근 가능
- **제로 오버헤드** — 모니터링 도구 자체의 CPU/메모리 사용 극소화, GPU 워크로드에 영향 없음
- **단일 바이너리** — 클라우드/컨테이너 환경에서 `scp` 또는 `COPY`만으로 배포

## License

MIT
