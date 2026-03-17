# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NVIDIA MIG(Multi-Instance GPU) 환경에서 제한되는 GPU 메트릭(VRAM 사용량, GPU Util, Mem Ctrl, SM 활용율 등)을 실시간 모니터링하는 TUI 프로그램. btop/nvtop 수준의 실시간 sparkline 그래프와 값 변화를 터미널에서 표시한다.

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
- 폴링 주기 기본 1000ms, 사용자 설정 가능 (`--interval`)
- GPU 인스턴스가 여러 개일 때 탭/스크롤로 전환
