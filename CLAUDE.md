# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NVIDIA MIG(Multi-Instance GPU) 환경에서 제한되는 GPU 메트릭(VRAM 사용량, GPU Util, Mem Ctrl, SM 활용율, Clock, PCIe, ECC, Throttle 등)을 실시간 모니터링하는 TUI 프로그램. btop/nvtop 수준의 실시간 sparkline 그래프와 값 변화를 터미널에서 표시한다.

## Tech Stack

- **Language**: Rust
- **GPU Metrics**: `nvml-wrapper` (NVIDIA NVML C API 바인딩) — MIG 인스턴스별 메트릭 수집
- **TUI Rendering**: `ratatui` + `crossterm` — 실시간 sparkline 그래프 위젯
- **System Metrics**: `sysinfo` — CPU 코어별 사용률, RAM/Swap

## Why Rust

- NVML API 직접 호출로 MIG 제한 우회 가능
- 모니터링 도구 자체의 CPU/메모리 오버헤드 최소화 (GPU 워크로드에 영향 없음)
- 단일 바이너리 배포 → 클라우드 환경에서 의존성 없이 배포

## Build & Run

```bash
cargo build --release
cargo run --release

# 개발 시
cargo run
cargo test
cargo clippy
cargo fmt --check
```

## Architecture

```
src/
  main.rs          — 진입점, 메인 루프 (collect → draw → event poll)
  app.rs           — 앱 상태 (메트릭, 히스토리, 선택 GPU)
  event.rs         — 키보드/틱 이벤트 핸들링
  gpu/
    mod.rs         — 모듈 선언
    nvml.rs        — NVML 래퍼 + MIG raw FFI + 디바이스 캐시
    metrics.rs     — GPU/시스템 메트릭 구조체 + VecDeque 링 버퍼 히스토리
  ui/
    mod.rs         — 모듈 선언
    dashboard.rs   — 전체 TUI 레이아웃 및 위젯 렌더링
```

## Key Design Decisions

- MIG 환경에서 `nvidia-smi`가 제공하지 않는 메트릭은 NVML `nvmlDeviceGetMigDeviceHandleByIndex` → `nvmlDeviceGetUtilizationRates` 등으로 직접 수집
- **3단계 MIG utilization 폴백**: (1) `utilization_rates()` → (2) `nvmlDeviceGetProcessUtilization()` → (3) `nvmlDeviceGetSamples(GPU_UTIL)` + MIG 슬라이스 비율 스케일링
- 드라이버 535.x에서 모든 표준 utilization API가 MIG에서 실패 → 부모 GPU의 `nvmlDeviceGetSamples` raw 값(/10000)을 MIG `gpuInstanceSliceCount` 비율로 환산
- `gpu_util`, `memory_util`은 `Option<u32>` — API 실패 시 0% 대신 "N/A" 표시로 오해 방지
- **VRAM carry-forward**: `memory_used`는 TTL 10회 제한 carry-forward, `memory_total`은 정적값이므로 `last_known_vram_total` 캐시로 무제한 복원 → 스파크라인 스케일(vram_max) 안정성 보장
- **스파크라인 TTL 후 push(0)**: carry-forward TTL 만료 시 push 중단 대신 0을 push → 그래프 동결 방지, 하강으로 데이터 소실 시각 표시
- **GPM 샘플 수명 관리**: `get_dram_bw_util_gpm`에서 조기 리턴 전 반드시 free, Drop은 `drain()` 패턴으로 double-free 방지
- **update_metrics O(1) 조회**: `prev_by_uuid` HashMap으로 이전 메트릭 O(1) 인덱스 참조, history는 `entry()` API로 단일 해시
- **GPM DRAM BW Util 폴백 + 오염 감지**: Hopper+ GPU에서 `nvmlGpmMigSampleGet()` → `NVML_GPM_METRIC_DRAM_BW_UTIL`로 MIG 메모리 컨트롤러 utilization 수집. Ampere에서는 GPM 미지원 → "N/A" 유지. 이전 tick 샘플과 현재 샘플 간 delta 계산 방식 (첫 tick은 None). Phase 1.5 GPM 호출 후 post-probe로 NVML 상태 오염 감지 → `gpm_disabled_parents`에 parent 등록, 이후 GPM 영구 비활성화로 VRAM 안정성 확보
- 모든 확장 메트릭(clock, PCIe, ECC, throttle 등)은 `.ok()`로 래핑 → MIG/vGPU에서 실패 시 `None`으로 graceful 처리
- 정적 메트릭(architecture, CC, temp thresholds, MIG slice count 등)은 `DeviceInfo` 캐시에 1회만 수집
- NVML 샘플 버퍼는 `RefCell<Vec<nvmlSample_t>>`로 grow-only 재사용 (tick당 할당 없음, ~2KB)
- 폴링 주기 기본 1000ms, 사용자 설정 가능 (`--interval`)
- GPU 인스턴스가 여러 개일 때 탭/스크롤로 전환
