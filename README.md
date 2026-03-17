# mig-gpu-mon

NVIDIA MIG(Multi-Instance GPU) 환경에서 `nvidia-smi`가 제공하지 못하는 GPU 메트릭을 실시간 모니터링하는 터미널 TUI 프로그램.

btop/nvtop 스타일의 실시간 그래프와 게이지를 터미널에서 표시하며, CPU 코어별 사용률과 시스템 RAM도 함께 모니터링한다.

## Screen Layout

### ASCII 구성도

```
┌─ mig-gpu-mon ──────────────────────────────────────────────────────────┐
│ MIG GPU Monitor | Driver: 535.129.03 | CUDA: 12.2 | GPUs: 3           │ ← Header
├─ CPU (64 cores) 23.4% ─────────┬─ Devices ────────────────────────────┤
│  0 ▮▮▯▯▯▯▯  12%  32 ▮▯▯▯▯  3% │ > MIG 0 (GPU 0: A100) GPU:45% MEM:… │ ↑
│  1 ▮▮▮▯▯▯▯  34%  33 ▮▮▯▯▯ 18% │   MIG 1 (GPU 0: A100) GPU:12% MEM:… │ │ 35%
│  2 ▮▮▮▮▯▯▯  52%  34 ▮▯▯▯▯  5% │   GPU 0: A100-SXM4-80GB             │ ↓
│  ...                            ├─ Detail ─────────────────────────────┤    ← Top 45%
├─ Memory ────────────────────────┤ Name: MIG 0 (GPU 0: A100-SXM4-80GB) │ ↑
│ RAM ▮▮▮▮▮▯▯ 89.2/256.0 GiB … │ UUID: MIG-a1b2c3d4e5f6...           │ │
│ SWP ▮▯▯▯▯▯▯  2.1/32.0 GiB  … │                                      │ │ 65%
│                                 │ VRAM: 12288 / 20480 MB (60.0%)      │ │
│                                 │ GPU Util: 45%                        │ │
│                                 │ Mem Util: 38%                        │ │
│                                 │ SM Util:  45%                        │ │
│                                 │ Temp: 62°C                           │ │
│                                 │ Power: 127.3W / 300.0W               │ │
│                                 │ Processes: 2                         │ ↓
├─ GPU Utilization % ─────────────┬─ CPU Total 23.4% ───────────────────┤
│ ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅            │ ▂▂▃▃▂▂▃▂▃▃▂▂▃▃▂▃                   │ ← 30%
├─ Memory Utilization % ──────────┼─ RAM 89.2/256.0 GiB (34.8%) ────────┤    ← Bottom 55%
│ ▃▃▃▄▄▅▅▅▄▃▃▃▄▄▅▅▄             │ ▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅                   │ ← 30%
├─ GPU % ───────┬─ VRAM ─────────┼─ RAM ────────────────────────────────┤
│ ████████░░ 45%│ ██████████ 60% │ ████████████████░░░░ 89.2/256.0 GiB │ ← 40% / Min(3)
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
│   │   │   │   └── 2-column core bars      "{idx} ▮▮▯▯ {pct}%"
│   │   │   └── RAM / Swap        Length(5)  " Memory "
│   │   │       ├── RAM line                 "RAM ▮▮▯▯ {used}/{total} GiB ({pct}%)"
│   │   │       └── SWP line                 "SWP ▮▮▯▯ {used}/{total} GiB ({pct}%)"
│   │   └── GPU Panel     50%
│   │       ├── Device List        35%       " Devices "
│   │       │   └── "{>} {MIG|GPU} {idx}: {name} | GPU:{pct}% MEM:{pct}%"
│   │       └── GPU Detail         65%       " Detail "
│   │           ├── Name:     {name}
│   │           ├── UUID:     {uuid (max 20 chars)}
│   │           ├── VRAM:     {used} / {total} MB ({pct}%)
│   │           ├── GPU Util: {pct}%
│   │           ├── Mem Util: {pct}%
│   │           ├── SM Util:  {pct}%          (MIG only)
│   │           ├── Temp:     {val}°C         (if available)
│   │           ├── Power:    {usage}W / {limit}W  (if available)
│   │           └── Processes: {count}
│   └── [Bottom 55%] ─── Horizontal ────────────────────────
│       ├── GPU Charts    50%
│       │   ├── GPU Utilization %   sparkline   30%
│       │   ├── Memory Utilization % sparkline  30%
│       │   └── Gauges row                      40%
│       │       ├── GPU %    gauge  50%
│       │       └── VRAM     gauge  50%
│       └── System Charts  50%
│           ├── CPU Total {pct}%   sparkline   45%
│           ├── RAM {u}/{t} GiB    sparkline   45%
│           └── RAM              gauge       Min(3)
└── Footer                          Length(3)
```

### 색상 코딩

| 요소 | 색상 | 조건 |
|------|------|------|
| CPU 코어 바 | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| RAM 바 | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| Swap 바 | DarkGray / Yellow / Red | 0-20% / 20-50% / 50%+ |
| GPU Util sparkline | Green | — |
| Mem Util sparkline | Blue | — |
| CPU sparkline | Cyan | — |
| RAM sparkline | Yellow | — |
| GPU % gauge | Green | — |
| VRAM gauge | Magenta | — |
| RAM gauge | Yellow | — |
| Temp | Green / Yellow / Red | 0-60°C / 60-80°C / 80°C+ |
| 선택된 GPU | Green + Bold | — |
| Header | Cyan + Bold | — |

## Why

MIG 환경에서 `nvidia-smi`는 GPU Utilization, Memory Utilization 등 핵심 메트릭을 표시하지 못한다.
`nvmlDeviceGetUtilizationRates()`가 MIG 디바이스 핸들에서 `NVML_ERROR_NOT_SUPPORTED`를 반환하기 때문이다.

이 도구는 NVML C API를 직접 호출하여 이 제한을 우회한다:

1. `nvmlDeviceGetMigDeviceHandleByIndex()` — MIG 인스턴스 핸들 획득
2. `nvmlDeviceGetProcessUtilization()` — 프로세스별 SM/Memory utilization 수집
3. 프로세스별 값을 집계하여 인스턴스 레벨의 GPU Util / Memory Util / SM Util 산출

## Features

- MIG 인스턴스별 GPU Util, Memory Util, SM Util, VRAM 사용량 실시간 표시
- 부모 GPU 메트릭(온도, 전력, 프로세스 수) 동시 표시
- CPU 코어별 사용률 (btop 스타일 2열 바 그래프)
- 시스템 RAM / Swap 사용량
- GPU Util / Memory Util / CPU Total / RAM 시계열 sparkline 그래프
- GPU Util / VRAM / RAM 게이지
- Tab/방향키로 GPU/MIG 인스턴스 전환
- 단일 바이너리 배포 (1.4MB, 의존성 없음)

## Requirements

- NVIDIA GPU + 드라이버 설치됨
- `libnvidia-ml.so.1` 접근 가능 (드라이버 설치 시 포함)
- 컨테이너 환경: `--gpus all` 또는 nvidia-docker 사용

## Build & Install

```bash
# 릴리즈 빌드 (최적화 + LTO + strip)
cargo build --release

# 바이너리 위치
ls -lh target/release/mig-gpu-mon  # ~1.4MB

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

MIG 환경에서 GPU/Memory utilization을 얻는 과정:

```
nvmlDeviceGetMigDeviceHandleByIndex(parent, idx)
    → mig_handle

nvmlDeviceGetUtilizationRates(mig_handle)
    → 성공: gpu_util, memory_util 사용
    → 실패 (NVML_ERROR_NOT_SUPPORTED):
        nvmlDeviceGetProcessUtilization(mig_handle, samples, &count, 0)
            → 1차 호출: count=0 → NVML_ERROR_INSUFFICIENT_SIZE, count에 필요 크기 반환
            → 2차 호출: 버퍼 전달 → 프로세스별 smUtil, memUtil 수집
            → max(smUtil), max(memUtil) 집계하여 인스턴스 레벨 값 산출
```

## Performance Optimization

모니터링 도구 자체가 GPU 워크로드에 영향을 주지 않도록 리소스 사용을 극소화했다.

### Memory

| 최적화 | 설명 |
|--------|------|
| `VecDeque` 링 버퍼 | 히스토리 저장 시 `Vec::remove(0)` O(n) → `VecDeque::pop_front()` O(1) |
| 디바이스 정보 캐시 | GPU name/uuid는 불변 — 첫 호출만 NVML API, 이후 `RefCell<HashMap>` 캐시 히트 |
| process sample 버퍼 재사용 | MIG util 수집 시 `RefCell<Vec>` grow-only 버퍼, 매 tick 할당/해제 없음 |
| CPU 사용률 버퍼 재사용 | `cpu_buf.clear()` + extend로 매 tick Vec 재할당 방지 |
| sparkline 변환 버퍼 | `thread_local!` scratch `Vec<u64>` — draw마다 4개 Vec 할당 제거 |
| `String::with_capacity` | `make_bar()` 바 문자열 pre-sized 할당, `.repeat()` 제거 |

### CPU

| 최적화 | 설명 |
|--------|------|
| `System::new()` | `System::new_all()` 대비 프로세스/디스크/네트워크 스캔 제거 |
| 타겟 refresh | `refresh_cpu_usage()` + `refresh_memory()`만 호출 |
| 기본 interval 1000ms | 500ms 대비 NVML+sysinfo 호출 횟수 절반 |
| CPU 첫 호출 priming | `sysinfo` 첫 `refresh_cpu_usage()` 0% 반환 문제 방지 |

### GPU (NVML 호출 최소화)

| 최적화 | 설명 |
|--------|------|
| `utilization_rates()` 우선 | MIG에서도 일단 시도, 실패 시에만 process util 폴백 |
| process util 2-pass | 1차로 count 확인, 2차로 데이터 수집 — 불필요한 대형 버퍼 할당 방지 |
| `RefCell` 내부 가변성 | `&self`로 NVML 핸들 빌려온 상태에서 캐시/버퍼 수정 가능 |

### Binary Size

| 설정 | 값 |
|------|-----|
| `opt-level` | 3 |
| `lto` | true (Link-Time Optimization) |
| `strip` | true (디버그 심볼 제거) |
| `tokio` 제거 | async 미사용 — 동기 이벤트 루프로 충분 |
| 최종 크기 | **~1.4MB** |

## Why Rust

- **NVML FFI 직접 호출** — MIG 제한 우회를 위한 raw C API 접근 가능
- **제로 오버헤드** — 모니터링 도구 자체의 CPU/메모리 사용 극소화, GPU 워크로드에 영향 없음
- **단일 바이너리** — 클라우드/컨테이너 환경에서 `scp` 또는 `COPY`만으로 배포

## License

MIT
