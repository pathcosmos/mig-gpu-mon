# mig-gpu-mon

[한국어](README.md) | **English**

A terminal TUI program for real-time monitoring of GPU metrics that `nvidia-smi` cannot provide in NVIDIA MIG (Multi-Instance GPU) environments.

Displays real-time sparkline graphs in btop/nvtop style, along with per-core CPU usage and system RAM monitoring.

> **Ubuntu-focused:** Development and testing are done on Ubuntu. Library search paths, error messages, and documentation are all written with Ubuntu as the primary target. It also works on RHEL-based distros, containers, and WSL2, but runs most smoothly on Ubuntu.

## Screen Layout

### ASCII Diagram

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
├─ Mem Ctrl 38% ──────────────────┤RAM ▮▮▮▮▮▮▯▯▯▯▯▯▯▯▯▯▯▯▯             │ ← RAM/SWP bars (no text)
│ ▃▃▃▄▄▅▅▅▄▃▃▃▄▄▅▅▄             │SWP ▮▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯▯             │    ← Bottom 55%
├─ VRAM 12288/20480 MB (60.0%) ──│ ▮ used  ▮ cached  ▮ free  RAM …     │ ← Memory legend (2 lines)
│ ▅▅▅▅▆▆▆▆▆▆▇▇▇▇▇▇▇             │ 70.1G/12.5G/6.6G  avl:77.5G        │
├─ PCIe TX:12.3 RX:56.7 MB/s ────├─ RAM ─────────────────────────────┤
│ ▂▃▃▅▅▆▅▃▂▂▃▅▆▆▅▃              │ ▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅ ← used+cached color│ (when PCIe available)
├────────────────────────────────────────────────────────────────────────┤
│ q Quit  Tab/↑↓ Switch GPU  [1/3]                                      │ ← Footer
└────────────────────────────────────────────────────────────────────────┘
```

### Layout Hierarchy

The actual layout tree from the code (`dashboard.rs`). Ratios correspond to `Constraint` values.

```
draw()
├── Header                          Length(3)
├── Main                            Min(10)
│   ├── [Top 45%]  ─── Horizontal ──────────────────────────
│   │   ├── System Panel  50%
│   │   │   └── CPU Cores         (full area) " CPU ({N} cores) {pct}% "
│   │   │       └── dynamic N-column bars   "{idx} ▮▮▯▯ {pct}%" (sorted by usage desc)
│   │   └── GPU Panel     50%
│   │       ├── Device List        20%       " Devices "
│   │       │   └── "{>} {MIG|GPU} {idx}: {name} | GPU:{pct}% Mem:{pct}%"
│   │       ├── GPU Detail         50%       " Detail "
│   │       │   ├── Name:      {name} [Parent: GPU {n}]   (MIG only)
│   │       │   ├── UUID:      {uuid}  Arch:{arch}  CC:{major.minor}
│   │       │   ├── VRAM      {used} MB / {total} MB ({pct}%)
│   │       │   ├── GPU: {pct}%  Mem: {pct}%  SM: {pct}%  (compact)
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
│       │   ├── GPU Util {pct}%        sparkline   25% (w/ PCIe) / 33%
│       │   ├── Mem Ctrl {pct}%        sparkline   25% / 33%
│       │   ├── VRAM {u}/{t} MB ({p}%) sparkline   25% / 34%
│       │   └── PCIe TX/RX MB/s       sparkline   25% (when PCIe data available)
│       └── System Charts  50%
│           ├── CPU Total {pct}%       sparkline   40%
│           ├── RAM/SWP Bars           Length(2)    bars only (no text values)
│           │   ├── RAM line                        "RAM ▮▮▮▮▯▯" (segmented: used/cached/free)
│           │   └── SWP line                        "SWP ▮▮▯▯"
│           ├── Memory Legend          Length(2)    2-line legend (above RAM chart)
│           │   ├── Line 1: "▮ used  ▮ cached  ▮ free  RAM {u}/{t} GiB ({p}%)"
│           │   └── Line 2: "{used}G/{cached}G/{free}G  avl:{avail}G"
│           └── RAM                    segmented chart Min(3)
│               └── segmented bar chart: used(Green/Yellow/Red) + cached(Blue), per-tick vertical bars
└── Footer                          Length(3)
```

### Color Coding

| Element | Color | Condition |
|---------|-------|-----------|
| CPU core bars | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| RAM bar (Used segment) | Green / Yellow / Red | 0-50% / 50-80% / 80%+ (based on total usage) |
| RAM bar (Cached segment) | Blue | Kernel cache/buffers (available - free) |
| RAM bar (Free segment) | DarkGray | Completely unused |
| RAM numeric (avl) | White | Available memory (usable without swapping) |
| Swap bar | DarkGray / Yellow / Red | 0-20% / 20-50% / 50%+ |
| GPU Util sparkline | Green | — |
| Mem Ctrl sparkline | Blue | — |
| VRAM sparkline | Magenta | — |
| PCIe sparkline | LightCyan | Shown only when PCIe data available |
| CPU sparkline | Cyan | — |
| RAM chart (Used segment) | Green / Yellow / Red | 0-50% / 50-80% / 80%+ (based on used%) |
| RAM chart (Cached segment) | Blue | Kernel cache/buffers (available - free) |
| VRAM % (Detail) | Green / Yellow / Red | 0-70% / 70-90% / 90%+ |
| Temp | Green / Yellow / Red | 0-60°C / 60-80°C / 80°C+ |
| Clock values | Cyan | — |
| PState | Green / Yellow / Red | P0 / P1-P4 / P5+ |
| PCIe info | LightCyan | — |
| Encoder/Decoder | Magenta | — |
| Throttle "None" | Green | Normal |
| Throttle active | Red + Bold | Warning |
| ECC errors 0 | Green | Normal |
| ECC uncorrected > 0 | Red + Bold | Critical |
| Selected GPU | Green + Bold | — |
| Header | Cyan + Bold | — |

## Why

In MIG environments, `nvidia-smi` cannot display key metrics such as GPU Utilization and Memory Utilization.
This is because `nvmlDeviceGetUtilizationRates()` returns `NVML_ERROR_NOT_SUPPORTED` for MIG device handles.

This tool bypasses that limitation through a **3-tier fallback mechanism** using the NVML C API directly:

1. **Tier 1:** `nvmlDeviceGetUtilizationRates()` — Standard API (works on non-MIG GPUs)
2. **Tier 2:** `nvmlDeviceGetProcessUtilization()` — Per-process SM/Memory utilization aggregation
3. **Tier 3:** `nvmlDeviceGetSamples(GPU_UTILIZATION_SAMPLES)` — Parent GPU sampling + MIG slice-ratio scaling

When all utilization APIs fail (common on driver 535.x with MIG), metrics are displayed as "N/A" instead of a misleading 0%.

## Features

- Real-time per-MIG-instance GPU Util, Mem Ctrl (memory controller / DRAM BW Util via GPM on Hopper+), SM Util, and VRAM usage
- **Top Processes** — displays top 5 processes by VRAM usage (PID, process name, MB); collects both compute and graphics processes, shows "N/A" when VRAM is unavailable
- Parent GPU metrics (temperature, power, process count) displayed simultaneously
- **Clock Speeds** — Graphics/SM/Memory clocks (MHz) + Performance State (P0~P15)
- **PCIe Throughput** — Gen/Width + TX/RX transfer rates (MB/s), conditional sparkline graph
- **Encoder/Decoder Utilization** — NVENC/NVDEC usage (%)
- **ECC Status** — enabled state + Corrected/Uncorrected error counts
- **Temperature Thresholds** — Slowdown/Shutdown threshold display
- **Throttle Reasons** — Real-time GPU throttle cause display (PwrCap, HW-Therm, etc.)
- **Architecture & Compute Capability** — GPU architecture (Ampere, Hopper, etc.) + CUDA CC
- Per-core CPU usage (sorted by usage descending, dynamic multi-column bar graph adapting to terminal width)
- System RAM (segmented bar: used/cached/free color-coded with per-segment numeric values + available/total) / Swap usage
  - RAM calculation: `used = total - available` (non-reclaimable), `cached = available - free` (reclaimable cache/buffers), `free = MemFree`
- Time-series sparkline graphs for GPU Util / Mem Ctrl / **VRAM** / **PCIe** / CPU Total + **RAM segmented chart** (used/cached color-coded, current values in title)
  - Unified graph direction: **RightToLeft** — newest data on the right, scrolling left over time (matches RAM segmented chart)
- Switch between GPU/MIG instances with Tab/arrow keys
- Single binary deployment (~1.5MB, dynamically links libc — no separate runtime install needed)

### MIG Environment Metric Availability

Some metrics are only available from the Parent GPU in MIG mode:

| Metric | MIG Instance | Parent GPU | Cloud vGPU |
|--------|-------------|-----------|-----------|
| GPU/Mem/SM Util | Yes (fallback) | Yes | Yes |
| VRAM | Yes | Yes | Yes |
| Architecture, CC | Yes | Yes | Yes |
| Clock Speeds | N/A | Yes | Yes |
| PCIe Throughput | N/A | Yes | Limited |
| Performance State | N/A | Yes | Yes |
| Temperature, Power | N/A | Yes | Yes |
| Temp Thresholds | N/A | Yes | Yes |
| ECC Status/Errors | N/A | Yes | Limited |
| Throttle Reasons | N/A | Yes | Limited |
| Encoder/Decoder | N/A | Yes | Yes |

## Requirements

- NVIDIA GPU with driver installed
- `libnvidia-ml.so.1` accessible (included with driver installation)
- Container environments: use `--gpus all` or nvidia-docker

### NVML Library Search Paths

At startup, the program searches the following paths in order to load the NVML library.
It automatically locates the library even in environments where `LD_LIBRARY_PATH` is not configured (containers, WSL, non-standard install paths).

| Order | Path | Target Environment |
|-------|------|--------------------|
| 0 | `--nvml-path` argument | User-specified (highest priority) |
| 0+ | Paths in `LD_LIBRARY_PATH` | Environment variable (cloud custom configs) |
| 1 | `libnvidia-ml.so.1` | Dynamic linker (standard) |
| 2 | `libnvidia-ml.so` | Default (symlink) |
| 3 | `/usr/lib/x86_64-linux-gnu/libnvidia-ml.so.1` | Debian / Ubuntu (x86_64) |
| 4 | `/usr/lib64/libnvidia-ml.so.1` | RHEL / CentOS / Rocky / Amazon Linux |
| 5 | `/usr/lib/aarch64-linux-gnu/libnvidia-ml.so.1` | Debian / Ubuntu (ARM64, Graviton) |
| 6 | `/usr/local/nvidia/lib64/libnvidia-ml.so.1` | NVIDIA Container Toolkit (vast.io, RunPod, EKS, GKE, AKS) |
| 7 | `/usr/local/nvidia/lib/libnvidia-ml.so.1` | NVIDIA Container Toolkit (alternate path) |
| 8 | `/run/nvidia/driver/usr/lib/x86_64-linux-gnu/libnvidia-ml.so.1` | NVIDIA GPU Operator (Kubernetes) |
| 9 | `/run/nvidia/driver/usr/lib64/libnvidia-ml.so.1` | NVIDIA GPU Operator (Kubernetes, RHEL) |
| 10 | `/usr/local/cuda/lib64/stubs/libnvidia-ml.so` | CUDA stubs (build-only) |
| 11 | `/usr/lib/wsl/lib/libnvidia-ml.so.1` | WSL2 |

### Per-Environment Setup Guide

| Environment | How to Run |
|-------------|------------|
| **Native (Ubuntu/RHEL)** | `mig-gpu-mon` (works immediately with driver installed) |
| **Docker** | `docker run --gpus all ...` or `--runtime=nvidia -e NVIDIA_DRIVER_CAPABILITIES=compute,utility` |
| **AWS (EC2 p4d/p5, EKS)** | Deep Learning AMI: works immediately. EKS: requires nvidia-device-plugin |
| **GCP (a2/a3 VM, GKE)** | GPU VM: works immediately. GKE: requires nvidia-driver-installer DaemonSet |
| **vast.io** | Auto-mounted in containers, works immediately |
| **RunPod** | Auto-mounted in containers, works immediately |
| **Lambda Labs** | Works immediately (native driver installed) |
| **WSL2** | Requires `wsl --install` followed by Windows NVIDIA driver installation |

### WSL2 Setup Guide

**Prerequisites:**
- Windows 11 (or Windows 10 21H2+)
- WSL2 (WSL1 does not support GPU)
- NVIDIA driver for Windows (not for Linux)

**Verification:**
1. In PowerShell: `wsl -l -v` → Confirm VERSION is 2
2. Inside WSL: `nvidia-smi` → Confirm GPU info is displayed
3. Inside WSL: `ls /usr/lib/wsl/lib/libnvidia-ml.so.1` → Confirm file exists

**Troubleshooting:**
- `nvidia-smi` not working → Update Windows NVIDIA driver
- Using WSL1 → Convert with `wsl --set-version <distro> 2`
- Library not found → Reinstall Windows NVIDIA driver

If automatic detection fails, specify the path manually:
```bash
mig-gpu-mon --nvml-path /custom/path/libnvidia-ml.so.1
```

## Quick Start (From Scratch)

From a fresh server — just run the **install script** and everything is handled automatically:

```bash
# Install git first if not present (Ubuntu: sudo apt install git / Rocky: sudo dnf install git)
git clone https://github.com/pathcosmos/mig-gpu-mon.git
cd mig-gpu-mon
./install.sh
```

What `install.sh` handles automatically:
1. Checks `sudo` availability (exits with clear message if non-root without sudo)
2. `curl` not installed → auto-installs (auto-detects apt/dnf/yum)
3. `gcc` (C linker) not installed → auto-installs `build-essential` (Ubuntu) or `gcc` (Rocky/RHEL)
4. `git` not installed → auto-installs
5. Rust not installed → auto-installs via rustup
6. `cargo build --release` → optimized build (LTO + strip, ~1.5MB)
7. Copies binary (`~/.cargo/bin` → `/usr/local/bin` → `~/.local/bin` fallback order) + verifies PATH

> Supports Ubuntu, Rocky Linux, CentOS, RHEL, and Amazon Linux. Package manager (apt/dnf/yum) is auto-detected.

After installation, run immediately:
```bash
mig-gpu-mon
```

### Manual Installation (Step by Step)

```bash
# 0. Build dependencies (Ubuntu)
sudo apt install -y curl git build-essential
# 0. Build dependencies (Rocky/RHEL)
# sudo dnf install -y curl git gcc

# 1. Install Rust (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. Download source
git clone https://github.com/pathcosmos/mig-gpu-mon.git
cd mig-gpu-mon

# 3. Build + register system-wide (single command)
cargo install --path .

# 4. Run
mig-gpu-mon
```

`cargo install` performs a release build (LTO + strip) and automatically registers the binary at `~/.cargo/bin/mig-gpu-mon`.
Since `~/.cargo/bin` is in `PATH`, you can run `mig-gpu-mon` from anywhere.

### One-Liner Install (Copy-Paste)

> **Prerequisite:** `curl`, `git`, and `gcc` must be installed. If not, run step 0 from Manual Installation above first.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source "$HOME/.cargo/env" && git clone https://github.com/pathcosmos/mig-gpu-mon.git /tmp/mig-gpu-mon && cargo install --path /tmp/mig-gpu-mon && mig-gpu-mon --help
```

### Copy Binary to Other Servers (No Rust Needed)

Copy the built binary to other servers with the same architecture (x86_64 Linux).
The target server's glibc version must be equal to or newer than the build server's (`ldd --version` to check).

```bash
# From the build server
scp target/release/mig-gpu-mon user@target-server:/usr/local/bin/

# On the target server (no Rust installation required)
mig-gpu-mon
```

### Uninstall

```bash
cargo uninstall mig-gpu-mon      # If installed via cargo install
# or
rm ~/.cargo/bin/mig-gpu-mon      # If installed via install.sh
rm /usr/local/bin/mig-gpu-mon    # If manually copied
```

## Build & Install (Detailed)

```bash
# Release build (optimized + LTO + strip)
cargo build --release

# Binary location
ls -lh target/release/mig-gpu-mon  # ~1.5MB

# Install to system path
cp target/release/mig-gpu-mon /usr/local/bin/

# Or run directly
./target/release/mig-gpu-mon
```

## Usage

```bash
# Default run (1-second interval)
mig-gpu-mon

# Adjust polling interval (milliseconds)
mig-gpu-mon --interval 2000    # 2-second interval (resource saving)
mig-gpu-mon -i 500             # 0.5-second interval (fast response)

# Manually specify NVML library path (when auto-detection fails)
mig-gpu-mon --nvml-path /usr/local/nvidia/lib64/libnvidia-ml.so.1

# Help
mig-gpu-mon --help
```

### Keyboard Controls

| Key | Action |
|-----|--------|
| `Tab` / `↓` / `→` | Next GPU/MIG instance |
| `Shift+Tab` / `↑` / `←` | Previous GPU/MIG instance |
| `q` / `Ctrl+C` | Quit |

## Tech Stack

| Role | Crate | Purpose |
|------|-------|---------|
| GPU Metrics | `nvml-wrapper` + `nvml-wrapper-sys` | NVML C API bindings, direct MIG FFI calls |
| TUI Rendering | `ratatui` + `crossterm` | Sparkline, gauge, layout widgets |
| System Metrics | `sysinfo` | Per-core CPU usage, RAM/Swap |
| CLI | `clap` | Argument parsing |
| Error Handling | `anyhow` | Error chaining |

## Architecture

```
src/
  main.rs           Entry point, main loop (collect → draw → event poll)
  app.rs            App state (metrics, history, selected GPU)
  event.rs          Keyboard / tick event handling
  gpu/
    mod.rs          Module declarations
    nvml.rs         NVML wrapper + MIG raw FFI + device cache
    metrics.rs      GPU/system metric structs + VecDeque ring buffer history
  ui/
    mod.rs          Module declarations
    dashboard.rs    Full TUI layout and widget rendering
```

### Main Loop Flow

```
loop {
    tick_start = Instant::now()
    1. Collect GPU metrics (NVML API)
       - Physical GPU: utilization_rates(), memory_info(), temperature(), ...
       - MIG instance: on utilization_rates() failure
         → fallback to nvmlDeviceGetProcessUtilization() for SM/Mem util aggregation
    2. Collect system metrics (sysinfo)
       - Per-core CPU usage, total RAM/Swap
    3. TUI rendering (ratatui)
    4. Reclaim CPU buffer (recover from previous SystemMetrics → zero-alloc)
    5. Wait for events (crossterm poll, blocks for interval - elapsed)
       - Drift-corrected: subtracts work time, polls only remaining duration
       - Process key input or tick → next loop iteration
}
```

## MIG Utilization Collection Mechanism

### 4-Tier Fallback Architecture

GPU/Memory utilization collection in MIG environments uses a cascading fallback strategy:

```
nvmlDeviceGetMigDeviceHandleByIndex(parent, idx)
    → mig_handle

Tier 1: nvmlDeviceGetUtilizationRates(mig_handle)
    → Success: use gpu_util, memory_util directly
    → Failure (NVML_ERROR_NOT_SUPPORTED): proceed to Tier 2

Tier 2: nvmlDeviceGetProcessUtilization(mig_handle, samples, &count, 0)
    → 1st call: count=0 → NVML_ERROR_INSUFFICIENT_SIZE, count returns required size
    → 2nd call: pass buffer → collect per-process smUtil, memUtil
    → Aggregate max(smUtil), max(memUtil) for instance-level values
    → If all samples are zero or fetch fails: proceed to Tier 3

Tier 3: nvmlDeviceGetSamples(parent_handle, GPU_UTILIZATION_SAMPLES)
    → Collect raw utilization samples from parent GPU (20ms intervals)
    → Average last 5 samples, divide by 10000 to get 0-100% scale
    → Scale by MIG slice ratio: mig_util = parent_util × total_slices / mig_slices
    → Example: parent=29%, MIG 3g.40gb → 29% × 7/3 ≈ 67%
    → If unavailable: display "N/A"

Tier 4: nvmlGpmMigSampleGet() — memory_util only (Hopper+ only)
    → Check GPM support from DeviceInfo cache
    → MIG: nvmlGpmMigSampleGet(parent_handle, gpuInstanceId, sample)
    → Regular GPU: nvmlGpmSampleGet(device, sample)
    → Compute nvmlGpmMetricsGet() with previous tick's sample + current sample
    → NVML_GPM_METRIC_DRAM_BW_UTIL (ID 10) → 0-100%
    → Ampere and older: GPM not supported → "N/A" maintained
```

### Driver 535.x MIG Limitations (Deep Investigation)

Extensive testing on **H100 PCIe + MIG 3g.40gb + Driver 535.129.03** revealed that **no standard NVML API** provides per-MIG-instance GPU utilization or memory controller utilization on this driver version.

#### NVML API Test Results (Driver 535.129.03)

| NVML API | Parent GPU | MIG Instance |
|---|---|---|
| `nvmlDeviceGetUtilizationRates()` | NotSupported (ret=3) | NotSupported (ret=3) |
| `nvmlDeviceGetProcessUtilization()` | Size query OK → fetch NotSupported | Size query OK → fetch InvalidArg (ret=2) |
| `nvmlDeviceGetSamples(GPU_UTIL)` | **Works** (119 samples, raw values) | InvalidArg (ret=2) |
| `nvmlDeviceGetSamples(MEM_UTIL)` | NotSupported | InvalidArg |
| `nvmlDeviceGetFieldValues(GPU_UTIL=203)` | FAIL (ret=2) | "OK" but val=0 (dummy data) |
| `nvmlDeviceGetFieldValues(MEM_UTIL=204)` | FAIL (ret=2) | "OK" but val=0 (dummy data) |

#### What Works on Each Device Type

| Metric | MIG Instance | Parent GPU |
|---|---|---|
| VRAM used/total | OK | NoPermission |
| Temperature | InvalidArg | OK |
| Power usage | NotSupported | OK |
| Clock speeds | NotSupported | OK |
| PCIe throughput | InvalidArg | OK |
| Process list | OK | - |
| `nvmlDeviceGetSamples(GPU_UTIL)` | InvalidArg | **OK** (raw values ~290000 range) |

#### Raw Sample Value Interpretation

`nvmlDeviceGetSamples(GPU_UTILIZATION_SAMPLES)` on the parent device returns:
- ~119 samples at 20ms intervals
- Values in ~230000-340000 range (not 0-100%)
- Dividing by 10000 yields plausible utilization percentages (~29%)
- Verified stable across multiple rounds (avg=291419, 292075, 292760)

#### MIG Slice-Ratio Scaling

Since per-MIG utilization is unavailable, the parent's aggregate utilization is scaled proportionally:
- MIG device attributes provide `gpuInstanceSliceCount` (e.g., 3 for 3g.40gb)
- `MaxMigDeviceCount` provides total slices (e.g., 7 for H100)
- Formula: `mig_util = parent_util × total_slices / mig_slices`
- Example: parent=29%, slices=3/7 → MIG util ≈ 67%

This is an **approximation** — it assumes all parent utilization comes from this MIG instance. Multiple active MIG instances would share the parent utilization.

#### Memory Controller Utilization — Exhaustive Investigation

All possible NVML APIs were investigated to collect Memory Controller utilization in MIG environments.

##### Attempted APIs and Results

| # | API / Approach | Parent GPU | MIG Instance | Verdict |
|---|---|---|---|---|
| 1 | `nvmlDeviceGetUtilizationRates().memory` | NotSupported | NotSupported | ❌ Officially unsupported on MIG |
| 2 | `nvmlDeviceGetProcessUtilization()` → `memUtil` | fetch fails | InvalidArg | ❌ Returns 0 or error |
| 3 | `nvmlDeviceGetSamples(MEM_UTIL)` | **NotSupported** | InvalidArg | ❌ Blocked on parent too → no scaling source |
| 4 | `nvmlDeviceGetFieldValues(MEM_UTIL=204)` | FAIL (ret=2) | "OK" but val=0 | ❌ Dummy data |
| 5 | `nvidia-smi dmon` mem% | — | Not supported for MIG | ❌ nvidia-smi limitation |
| 6 | CUDA `cudaMemGetInfo` | Capacity only | Capacity only | ❌ Not controller utilization |
| 7 | `nvmlDeviceGetMemoryBusWidth` | Static value | Static value | ❌ Bus width (bits), not utilization |
| 8 | Driver 545/550/555 | — | — | ❌ No standard API change |
| 9 | **NVML GPM `DRAM_BW_UTIL`** | — | **Works on Hopper+** | ✅ Only viable path |

##### Key Difference from GPU Util

GPU Util has a working fallback because `nvmlDeviceGetSamples(GPU_UTIL)` **works on the parent GPU**, enabling MIG slice-ratio scaling. However, `MEM_UTIL` is **NotSupported even on the parent GPU**, so there is no source data to scale from.

##### GPM (GPU Performance Monitoring) — Hopper+ Only Solution

The NVML GPM API was introduced in driver 520+ for **Hopper (H100) and newer** architectures. `NVML_GPM_METRIC_DRAM_BW_UTIL` (metric ID 10) reports DRAM bandwidth utilization as a percentage of theoretical maximum (0.0–100.0%), and is collectible on MIG instances via `nvmlGpmMigSampleGet()`.

```
GPM Collection Flow:
1. nvmlGpmQueryDeviceSupport(parent_device) → check GPM support (cached in DeviceInfo)
   ⚠ Must use parent GPU handle — MIG handles return errors AND corrupt NVML state
2. nvmlGpmSampleAlloc() → allocate sample buffer
3. nvmlGpmMigSampleGet(parent, gpuInstanceId, sample) — for MIG instances
   nvmlGpmSampleGet(device, sample) — for regular GPUs (non-MIG only)
   ⚠ Never call nvmlGpmSampleGet on MIG handles — corrupts NVML state → breaks subsequent queries
4. nvmlGpmMetricsGet() with previous tick's sample + current sample
5. metrics[0].value → DRAM BW Util (0.0–100.0%)

collect_mig_instances 2-phase collection (v0.3):
  Phase 1: Collect VRAM + utilization for all MIG instances (no GPM calls)
           → memory_info(), utilization_rates(), process util, sample scaling
  Phase 2: GPM fallbacks for memory_util (Mem Ctrl) collection (Hopper+ only)
           → nvmlGpmMigSampleGet(parent, gi_id, sample)
  Purpose: Even if GPM calls corrupt NVML state, VRAM is already collected in Phase 1
```

| GPU Architecture | GPM Support | Mem Ctrl Display |
|---|---|---|
| Ampere (A100/A30) | ❌ | "N/A" maintained |
| Hopper (H100/GH200) | ✅ | DRAM BW Util % |
| Blackwell+ | ✅ | DRAM BW Util % |

> **Implementation Status:** This tool already implements GPM DRAM BW Util collection, automatically enabled on Hopper+ GPUs. First tick collects baseline (None), actual values displayed from 2nd tick onwards.

> **Note:** NVIDIA driver 550+ (CUDA 12.4+) added proper `nvmlDeviceGetUtilizationRates()` support for MIG device handles, making all 3 fallback tiers unnecessary.

#### VRAM + Mem Ctrl Simultaneous Display Bug Analysis & Fix (v0.3)

##### Symptoms

In MIG environments, VRAM was displayed only on the first tick, then silently dropped to `0/0 MB`. Meanwhile, Mem Ctrl showed "N/A" or an actual value. Both metrics should be displayed simultaneously.

##### Root Cause Analysis — 3 Cascading Bugs

**Bug 1: `get_device_info` GPM query corrupts NVML state on MIG handles**

```
collect_device_metrics() call order (before fix):
  line 543: get_device_info(mig_device)
            → nvmlGpmQueryDeviceSupport(mig_handle)  ← corrupts NVML state!
  line 546: memory_info()                             ← queries VRAM in corrupted state → fails
```

`get_device_info()` called `nvmlGpmQueryDeviceSupport()` on MIG handles. This GPM query corrupted NVML driver internal state, causing the subsequent `memory_info()` VRAM query to fail or return `(0, 0)`. Although cached after the first call (DeviceInfo cache), it combined with Bug 2 for persistent damage.

**Bug 2: Cross-tick GPM state corruption (core mechanism)**

```
Tick N:   VRAM query (succeeds) → GPM fallback (nvmlGpmMigSampleGet) → NVML state corrupted
Tick N+1: VRAM query (fails — residual corruption from Tick N GPM call) → GPM fallback → corrupts again
Tick N+2: VRAM query (fails) → ...
```

The GPM fallback (`nvmlGpmMigSampleGet`) in `collect_mig_instances` executed after the VRAM query within the same tick, but the GPM call corrupted NVML driver state that **persisted across ticks**. Reordering within the same function was insufficient — corruption survived between ticks.

**Bug 3: `memory_used`/`memory_total` silently masked failures**

```rust
// Before fix: unwrap_or((0, 0)) — failure silently becomes 0/0
let (memory_used, memory_total) = device.memory_info()
    .map(|m| (m.used, m.total))
    .unwrap_or((0, 0));  // ← "VRAM 0/0 MB (0.0%)" — user perceives as "disabled"
```

When the VRAM query failed, the `u64` type fell back to `(0, 0)`, showing "VRAM 0/0 MB (0.0%)". This contrasted with `memory_util` (`Option<u32>`) which explicitly showed "Mem Ctrl N/A". From the user's perspective, VRAM appeared to "disappear."

##### Timeline Reproduction

```
Tick 1 (first tick):
  ├── get_device_info(mig) → nvmlGpmQueryDeviceSupport(mig_handle) [first call, cache miss]
  │   → possible NVML state corruption (but cached, no repeat calls)
  ├── memory_info() → succeeds or fails depending on corruption severity
  ├── utilization_rates() → NVML_ERROR_NOT_SUPPORTED (MIG limitation)
  ├── process_util fallback → collects sm/mem util
  └── GPM fallback → nvmlGpmMigSampleGet(parent, gi_id) → first tick, no prev_sample → None
      → but the GPM call itself corrupts NVML state

Tick 2 (subsequent ticks):
  ├── get_device_info(mig) → cache hit (no GPM query)
  ├── memory_info() → FAILS (residual corruption from Tick 1 GPM call)
  │   → unwrap_or((0, 0)) → VRAM 0/0 MB ← what the user sees as "disabled"
  ├── ... (rest unchanged)
  └── GPM fallback → nvmlGpmMigSampleGet → has prev_sample → returns memory_util value!
      → but corrupts NVML state again → Tick 3 VRAM also fails

Result: VRAM displays on Tick 1 only, shows 0/0 MB from Tick 2 onwards
        Mem Ctrl displays value from Tick 2+ (or always N/A on Ampere)
```

##### Fix Details (3 changes)

**Fix 1: Block GPM query on MIG handles in `get_device_info`** (`nvml.rs`)

```rust
// Before: GPM query on all devices
fn get_device_info(&self, device: &Device) -> DeviceInfo {
    gpm_supported: nvmlGpmQueryDeviceSupport(device.handle(), ...)  // MIG handle → corruption!
}

// After: skip_gpm_query parameter added
fn get_device_info(&self, device: &Device, skip_gpm_query: bool) -> DeviceInfo {
    gpm_supported: if skip_gpm_query { false } else { nvmlGpmQueryDeviceSupport(...) }
}
// MIG handles: get_device_info(mig_device, true)  → GPM query skipped
// Parent:      get_device_info(parent_device, false) → GPM query runs normally
```

**Fix 2: 2-phase separation in `collect_mig_instances`** (`nvml.rs`)

```rust
// Before: VRAM + GPM interleaved per MIG instance
for mig in mig_instances {
    metrics = collect_device_metrics(mig)  // VRAM query
    gpm_fallback(mig)                      // GPM call → corrupts next MIG's VRAM query
}

// After: 2-phase separation
// Phase 1: Collect all MIG VRAM (no GPM calls)
for mig in mig_instances {
    metrics = collect_device_metrics(mig)  // VRAM + utilization + process util
    phase1.push(metrics)
}
// Phase 2: GPM fallbacks (all VRAM already collected)
for metrics in &mut phase1 {
    gpm_fallback(metrics)  // GPM call → corruption doesn't affect VRAM
}
```

**Fix 3: `memory_used`/`memory_total` → `Option<u64>`** (`metrics.rs` + `dashboard.rs`)

```rust
// Before: u64 — failures masked as (0, 0)
pub memory_used: u64,
pub memory_total: u64,
// → "VRAM 0/0 MB (0.0%)" — confusing to users

// After: Option<u64> — failures shown as "N/A"
pub memory_used: Option<u64>,
pub memory_total: Option<u64>,
// → "VRAM N/A" — consistent with gpu_util, memory_util pattern
```

UI updated simultaneously:
- Detail panel: displays `VRAM N/A` (DarkGray color)
- Sparkline title: displays `VRAM N/A`
- History: pushes only on `Some` → prevents graph data corruption on failed ticks

##### Modified Files

| File | Changes |
|------|---------|
| `src/gpu/nvml.rs` | Added `skip_gpm_query` parameter to `get_device_info`, 2-phase separation in `collect_mig_instances`, MIG callers pass `skip_gpm_query=true` |
| `src/gpu/metrics.rs` | `memory_used`/`memory_total` → `Option<u64>`, `memory_used_mb()`/`memory_total_mb()`/`memory_percent()` → return `Option` |
| `src/ui/dashboard.rs` | Added `N/A` fallback to VRAM detail/sparkline, `vram_max` uses `and_then` |

##### Cross-Verification Matrix

| Scenario | VRAM Display | Mem Ctrl Display | Verification |
|----------|-------------|-----------------|-------------|
| Hopper+ MIG, Tick 1 | Normal value (Phase 1) | N/A (GPM first tick, no prev_sample) | VRAM collected in Phase 1 → Phase 2 GPM corruption irrelevant |
| Hopper+ MIG, Tick 2+ | Normal value (Phase 1) | Normal value (Phase 2 GPM delta) | Even with GPM corruption, VRAM already completed in Phase 1 |
| Ampere MIG | Normal value | N/A (GPM unsupported) | No GPM calls at all → no VRAM corruption possible |
| Non-MIG GPU | Normal value | Normal or GPM value | GPM called only for non-MIG, VRAM collected first |
| memory_info() failure | "VRAM N/A" | Separate path | Option<u64> provides explicit failure display |
| get_device_info first call (MIG) | Normal value | — | skip_gpm_query=true → GPM query skipped → no NVML corruption |

#### VRAM Stagnation Bug Analysis & Fix (v0.3.1)

##### Symptoms

In MIG environments, VRAM displays correctly for the first few ticks, then becomes **stagnant** — the text value freezes and the sparkline graph stops updating.

##### Root Cause Analysis — 2 Cascading Bugs

**Bug 1: Cross-tick GPM corruption causes `memory_info()` failure**

The v0.3 2-phase separation prevented GPM→VRAM corruption **within the same tick**, but the NVML driver state corruption left by GPM calls **persists across ticks**.

```
Tick N:   Phase 1 (VRAM succeeds) → Phase 2 (GPM call → NVML state corrupted)
Tick N+1: Phase 1 (memory_info() fails — residual corruption from prev tick) → memory_used = None
Tick N+2: Phase 1 (fails again) → ...
```

2-phase only provides intra-tick protection. **Cross-tick** corruption persistence requires separate handling.

**Bug 2: `MetricsHistory::push()` skips on `None` → sparkline freezes**

```rust
// Before: skips push entirely when None
if let Some(val) = metrics.memory_used_mb() {
    Self::push_ring(&mut self.memory_used_mb, val, self.max_entries);
}
// → when memory_info() fails, ring buffer stops updating → sparkline frozen
```

When `memory_used` is `None`, nothing is pushed to the `memory_used_mb` ring buffer, causing the sparkline to freeze at the last successful value. The same issue affected all sparkline metrics: `gpu_util`, `memory_util`, `temperature`, etc.

##### Fix Details (2 changes)

**Fix 1: VRAM carry-forward in `update_metrics()`** (`app.rs`)

```rust
// Before: stored as-is
pub fn update_metrics(&mut self, new_metrics: Vec<GpuMetrics>) { ... }

// After: when memory_used is None, inherit from previous tick's same UUID
pub fn update_metrics(&mut self, mut new_metrics: Vec<GpuMetrics>) {
    for m in &mut new_metrics {
        if m.memory_used.is_none() {
            if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) {
                m.memory_used = prev.memory_used;
                m.memory_total = prev.memory_total;
            }
        }
    }
    // ... existing logic
}
```

- Even when `memory_info()` fails due to GPM corruption, text display doesn't drop to "N/A"
- UUID-based matching prevents cross-instance confusion

**Fix 2: `push_or_repeat()` for all sparkline metrics** (`metrics.rs`)

```rust
// Before: skips push on None
if let Some(val) = metrics.gpu_util {
    Self::push_ring(&mut self.gpu_util, val, self.max_entries);
}

// After: repeats last value on None → sparkline keeps rolling
fn push_or_repeat<T: Copy>(buf: &mut VecDeque<T>, val: Option<T>, max: usize) {
    let v = match val {
        Some(v) => v,
        None => match buf.back() {
            Some(&last) => last,
            None => return,  // never observed — don't fabricate data
        },
    };
    Self::push_ring(buf, v, max);
}
```

Applied uniformly to all sparkline metrics: `gpu_util`, `memory_util`, `memory_used_mb`, `sm_util`, `temperature`, `power_usage_w`, `clock_graphics_mhz`, `pcie_tx_kbps`, `pcie_rx_kbps`.

##### Modified Files

| File | Changes |
|------|---------|
| `src/app.rs` | `update_metrics()` — VRAM carry-forward (inherit from previous tick's same UUID) |
| `src/gpu/metrics.rs` | `push_or_repeat()` — repeat last value on None for all sparkline metrics |

##### Cross-Verification Matrix

| Scenario | VRAM Text | VRAM Sparkline | Verification |
|----------|-----------|---------------|-------------|
| Tick 1 (normal) | Normal value | Normal push | Phase 1 collection succeeds |
| Tick 2+ (memory_info fails from GPM corruption) | Previous value (carry-forward) | Last value repeated (rolling) | update_metrics inheritance + push_or_repeat |
| Tick 2+ (memory_info recovers) | New value shown | New value pushed | Carry-forward only activates on None |
| GPU util temporarily None | Last value retained | Last value repeated | push_or_repeat applied |
| Never-observed metric | N/A | No push | `buf.back() == None` → return, prevents data fabrication |
| Ampere MIG (no GPM) | Normal value | Normal push | No GPM calls → no corruption possible |
| Non-MIG GPU | Normal value | Normal push | memory_info() works normally |

##### Relationship to v0.3 Fixes

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3 Fix 1: `skip_gpm_query` | Blocks GPM query on MIG handles → prevents first-call corruption | `get_device_info()` |
| v0.3 Fix 2: 2-phase separation | Blocks GPM→VRAM corruption within same tick | `collect_mig_instances()` |
| v0.3 Fix 3: `Option<u64>` | Shows "N/A" instead of 0/0 on VRAM failure | `metrics.rs` |
| **v0.3.1 Fix 1: carry-forward** | **Inherits VRAM value when cross-tick GPM corruption persists** | `app.rs:update_metrics()` |
| **v0.3.1 Fix 2: push_or_repeat** | **Prevents sparkline stagnation on None values** | `metrics.rs:push()` |

v0.3 provides "corruption prevention" defense; v0.3.1 adds a "resilience" layer that maintains display even when corruption occurs.

#### Top Processes Missing Bug Analysis & Fix (v0.3.2)

##### Symptoms

In MIG environments, the "Top Processes" panel shows only "No compute processes" even when processes are actively running on the GPU.

##### Root Cause Analysis — 3 Issues

**Issue 1: `UsedGpuMemory::Unavailable` processes completely filtered out**

```rust
// Before: processes without VRAM info were entirely excluded
let mut entries: Vec<(u32, u64)> = procs
    .iter()
    .filter_map(|p| {
        let vram = match p.used_gpu_memory {
            UsedGpuMemory::Used(bytes) => bytes,
            UsedGpuMemory::Unavailable => return None,  // ← process lost!
        };
        Some((p.pid, vram))
    })
    .collect();
```

In MIG environments (especially driver 535.x), NVML returns `Unavailable` for process VRAM, causing `filter_map` to drop the process entirely from the UI.

**Issue 2: Only compute processes collected**

```rust
// Before: only compute processes
let (process_count, top_processes) = match device.running_compute_processes() { ... };
// → graphics processes (Vulkan/OpenGL) never collected
```

`running_graphics_processes()` was never called, so non-CUDA graphics processes were missing.

**Issue 3: API error returns empty list silently**

```rust
// Before: error → empty list, no fallback
Err(_) => (0, Vec::new()),
```

When `running_compute_processes()` failed, an empty list was returned, making it appear that no processes existed.

##### Fix Details (3 changes)

**Fix 1: `GpuProcessInfo.vram_used` → `Option<u64>`** (`metrics.rs`)

```rust
// Before: u64 — Unavailable processes excluded
pub vram_used: u64,

// After: Option<u64> — Unavailable preserved as None
pub vram_used: Option<u64>,

pub fn vram_used_mb(&self) -> Option<u64> {
    self.vram_used.map(|v| v / (1024 * 1024))
}
```

**Fix 2: Unified compute + graphics process collection** (`nvml.rs`)

```rust
// After: collect from both APIs, dedup by PID
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

- Both APIs use independent `if let Ok` — one failing doesn't block the other
- `HashSet<u32>` prevents duplicate PIDs (processes in both compute and graphics)

**Fix 3: VRAM "N/A" display** (`dashboard.rs`)

```rust
// After: show "N/A" when VRAM unavailable (process still displayed)
let vram_str = match proc.vram_used_mb() {
    Some(mb) => format!("{:>7} MB", mb),
    None => format!("{:>10}", "N/A"),
};
```

##### Sorting Logic Improvement

```rust
// Known VRAM descending → Unavailable at end → stable by PID
entries.sort_by(|a, b| match (b.1, a.1) {
    (Some(bv), Some(av)) => bv.cmp(&av),
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (None, None) => a.0.cmp(&b.0),
});
entries.truncate(5);
```

##### Modified Files

| File | Changes |
|------|---------|
| `src/gpu/metrics.rs` | `GpuProcessInfo.vram_used` → `Option<u64>`, `vram_used_mb()` → `Option<u64>` |
| `src/gpu/nvml.rs` | Unified compute + graphics collection, PID dedup, `Unavailable` → `None` preserved, independent error handling |
| `src/ui/dashboard.rs` | VRAM "N/A" display, "No processes" message updated |

##### Cross-Verification Matrix

| Scenario | Process Display | VRAM Display | Verification |
|----------|----------------|-------------|-------------|
| Compute process + VRAM available | Shown | MB value | Existing behavior preserved |
| Compute process + VRAM Unavailable | Shown | "N/A" | Fix 1: Option<u64> |
| Graphics process only | Shown | MB or "N/A" | Fix 2: graphics added |
| Same PID in both APIs | 1 entry only | No duplicates | HashSet dedup |
| Compute API fails, graphics OK | Graphics only shown | MB or "N/A" | Independent if let Ok |
| Both APIs fail | "No processes" | — | Empty entries |
| More than 5 processes | Top 5 by VRAM | MB first, N/A last | Sort logic |
| MIG instance | MIG-specific processes | MB or "N/A" | Collected via MIG handle |

#### MIG Top Processes Parent Device Fallback Bug Analysis & Fix (v0.3.3)

##### Symptoms

Despite v0.3.2 fixes (preserving `UsedGpuMemory::Unavailable` processes and unified compute+graphics collection), MIG instances still show "No processes" even when processes are actively running.

##### Root Cause Analysis

**Issue: `running_compute_processes()` / `running_graphics_processes()` fail on MIG device handles**

```rust
// Inside collect_device_metrics() — called with MIG handle
if let Ok(procs) = device.running_compute_processes() {   // ← Returns Err on MIG handle
    for p in &procs { ... }
}
if let Ok(procs) = device.running_graphics_processes() {  // ← Returns Err on MIG handle
    for p in &procs { ... }
}
```

On NVIDIA drivers like 535.x, `nvmlDeviceGetComputeRunningProcesses()` / `nvmlDeviceGetGraphicsRunningProcesses()` return `NVML_ERROR_NOT_SUPPORTED` for **MIG device handles**. The `if let Ok` pattern silently swallows the error, leaving `entries` empty → "No processes" displayed.

However, calling the same APIs on the **parent GPU device handle** returns all processes across all MIG instances, with the `gpu_instance_id` field properly set.

##### Fix Details

**Added Phase 3 to `collect_mig_instances()`** (`nvml.rs`)

```rust
// === Phase 3: MIG process parent device fallback ===
// When Phase 1 fails to collect processes via MIG handle,
// query parent GPU and filter by gpu_instance_id

// 1. Query compute + graphics processes from parent device (once)
let parent_procs = parent_device.running_compute_processes()
    + parent_device.running_graphics_processes();  // PID dedup

// 2. Distribute to MIG instances by gpu_instance_id matching
for (mig_handle, metrics) in &mut phase1 {
    if !metrics.top_processes.is_empty() { continue; }  // Skip if already collected
    let gi_id = get_device_info(mig_device).gpu_instance_id;
    metrics.top_processes = parent_procs
        .filter(|p| p.gpu_instance_id == gi_id)
        .sort_by_vram_desc()
        .truncate(5);
}
```

**Key Design:**
- `nvml-wrapper 0.10`'s `ProcessInfo` struct provides `gpu_instance_id: Option<u32>` → enables per-MIG-instance filtering
- MIG instances that already collected processes in Phase 1 are skipped (some drivers do support process queries on MIG handles)
- Parent device query runs only once → minimal additional NVML IPC per tick

##### Full 3-Phase Collection Flow

```
collect_mig_instances():
  Phase 1: Collect base metrics for each MIG instance (VRAM, util, process attempt)
           → Process API on MIG handle fails → top_processes = []
  Phase 2: GPM fallbacks (memory_util, Hopper+ only)
           → All VRAM already collected → GPM corruption irrelevant
  Phase 3: Process parent device fallback (NEW)
           → Query processes from parent GPU → filter by gpu_instance_id → distribute to MIG instances
```

##### Modified Files

| File | Changes |
|------|---------|
| `src/gpu/nvml.rs` | Added Phase 3 to `collect_mig_instances()` — parent device process query + `gpu_instance_id` filtering + per-MIG-instance distribution |

##### Cross-Verification Matrix

| Scenario | Process Display | VRAM Display | Verification |
|----------|----------------|-------------|-------------|
| MIG handle process API succeeds | Collected directly in Phase 1 | MB or "N/A" | `!top_processes.is_empty()` → Phase 3 skipped |
| MIG handle process API fails (535.x) | Collected from parent device → gi_id filter | MB or "N/A" | Phase 3 fallback activates |
| Parent device process API also fails | "No processes" | — | Both APIs fail → empty list |
| Processes distributed across multiple MIG instances | Correctly distributed per MIG | MB or "N/A" | `gpu_instance_id` matching ensures accurate distribution |
| Non-MIG GPU | Phase 3 not executed | MB value | `collect_mig_instances` not called at all |

##### v0.3.2 → v0.3.3 Defense Layer Relationship

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3.2: VRAM Unavailable preservation | Keeps process visible when VRAM is `None` | `collect_device_metrics()` |
| v0.3.2: compute + graphics unification | Collects from both APIs, PID dedup | `collect_device_metrics()` |
| **v0.3.3: Parent device fallback** | **When MIG handle process API fails → collect from parent → distribute by gi_id** | `collect_mig_instances()` Phase 3 |

v0.3.2 prevents "data loss when MIG handles do return processes"; v0.3.3 handles "when MIG handle process APIs fail entirely, fall back to parent device".

#### Top Processes Flicker + Resource Audit Fix (v0.3.4)

##### Symptoms

In MIG environments, Top Processes briefly appears for one tick then disappears (flicker). Long-running operation could accumulate orphan HashMap entries due to pruning condition bug.

##### Root Cause Analysis — 3 Issues

**Issue 1: No carry-forward for Top Processes**

`memory_used` has carry-forward when API fails, but `top_processes` had no equivalent protection.

```
Tick 1: MIG device running_compute_processes() succeeds → processes shown
Tick 2: Same API fails → top_processes empty → Phase 3 attempted
Phase 3: gpu_instance_id unavailable → all processes filtered → "No processes"
Tick 3: API succeeds again → process flicker
```

**Issue 2: Phase 3 gpu_instance_id filter overly strict**

```rust
// Before: processes with gpu_instance_id=None entirely filtered out
.filter(|(_, _, proc_gi)| *proc_gi == gi_id && gi_id.is_some())
```

On drivers where parent GPU processes don't have `gpu_instance_id` set, Phase 3 fallback was ineffective.

**Issue 3: app.rs history HashMap pruning condition bug**

```rust
// Before: only prunes when GPU count decreases → MIG reconfig with same count leaks entries
if self.history.len() > new_metrics.len() { ... }
```

4 MIG → different 4 MIG reconfig: old 4 entries + new 4 entries = 8 entries accumulated.

##### Fix Details (5 changes)

**Fix 1: Top Processes carry-forward** (`app.rs`)

```rust
// Retain previous tick's processes on NVML API intermittent failure
if m.top_processes.is_empty() {
    if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) {
        if !prev.top_processes.is_empty() {
            m.top_processes = prev.top_processes.clone();
            m.process_count = prev.process_count;
        }
    }
}
```

**Fix 2: Phase 3 gpu_instance_id fallback relaxation** (`nvml.rs`)

```rust
// Show all parent processes when no gpu_instance_id available
let any_gi_available = parent_procs.iter().any(|(_, _, gi)| gi.is_some());

let entries = if any_gi_available && gi_id.is_some() {
    // Normal path: filter by matching gpu_instance_id
    parent_procs.filter(|p| p.gpu_instance_id == gi_id)
} else {
    // Fallback: show all parent processes (better than showing nothing)
    parent_procs.all()
};
```

**Fix 3: HashMap pruning accuracy improvement** (`app.rs`)

```rust
// Before: only len comparison → missed UUID changes with same GPU count
// After: detect UUID mismatch + always prune + shrink_to()
if self.history.len() != new_metrics.len()
    || self.history.keys().any(|uuid| !new_metrics.iter().any(|m| m.uuid == *uuid))
{
    self.history.retain(...);
    if self.history.capacity() > target * 2 {
        self.history.shrink_to(target);
    }
}
```

**Fix 4: proc_name_cache shrink threshold improvement** (`nvml.rs`)

```rust
// Before: len.max(16) * 4 → capacity held at 4000 even with 10 PIDs remaining
// After: target * 2 → more aggressive memory reclaim
let target = name_cache.len().max(16) * 2;
if name_cache.capacity() > target * 2 {
    name_cache.shrink_to(target);
}
```

**Fix 5: datetime format cache** (`dashboard.rs`)

```rust
// thread_local cache — re-format only when second changes
thread_local! {
    static TIME_CACHE: RefCell<(i64, String)> = RefCell::new((0, String::new()));
}
// Repeated calls within same second return cached value → saves 1 String alloc/tick
```

##### Modified Files

| File | Changes |
|------|---------|
| `src/app.rs` | Top Processes carry-forward + HashMap pruning condition fix + shrink_to() |
| `src/gpu/nvml.rs` | Phase 3 gpu_instance_id fallback relaxation + proc_name_cache shrink threshold improvement |
| `src/ui/dashboard.rs` | datetime format thread_local cache |

##### Cross-Verification Matrix

| Scenario | Top Processes Display | Resource Impact | Verification |
|----------|----------------------|----------------|-------------|
| MIG process API intermittent failure | Previous tick value retained (carry-forward) | 1 Clone/tick (5 processes) | Activates only on API failure |
| Parent GPU lacks gpu_instance_id | All parent processes shown | Same as existing | Fallback path activated |
| MIG reconfig (UUID change, same GPU count) | — | Orphan entries removed immediately | UUID mismatch detection |
| Mass PID death (1000→10) | — | Capacity aggressively shrunk | target*2 threshold |
| Long-running at 1s interval | Normal | RSS ~4-8MB stable | All buffers bounded |

##### v0.3.3 → v0.3.4 Defense Layer Relationship

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3.3: Parent device fallback | MIG handle process API fail → collect from parent | `collect_mig_instances()` Phase 3 |
| **v0.3.4: carry-forward** | **Phase 3 also returns empty → retain previous tick** | `app.rs:update_metrics()` |
| **v0.3.4: gi_id fallback relaxation** | **Parent processes lack gi_id → show all** | `nvml.rs` Phase 3 filter |
| **v0.3.4: HashMap pruning accuracy** | **UUID change detection + shrink** | `app.rs:update_metrics()` |
| **v0.3.4: resource audit optimizations** | **proc_name_cache shrink + datetime cache** | `nvml.rs`, `dashboard.rs` |

v0.3.3 adds "process collection fallback path"; v0.3.4 improves "fallback path robustness + display stability + long-running resource optimization".

#### Sparkline Direction Bug + Long-Running Optimization (v0.3.5)

##### Symptoms

1. Sparkline graphs display **oldest data** instead of newest, with the time axis reversed
2. CPU sparkline f32→u64 conversion truncates instead of rounding (99.7% → 99)
3. PCIe sparkline title shows both TX/RX values but graph only renders TX — user confusion
4. Process name cache returns `String::clone()` every tick — unnecessary heap allocation in long-running operation
5. `throttle_reasons` allocates a new `String` every tick — even for frequent single values like "None"

##### Root Cause Analysis

**Bug 1: Misunderstanding of ratatui `RenderDirection::RightToLeft` semantics (Critical)**

ratatui's `RightToLeft` direction places `data[0]` at the **right edge**:
```
data = [0, 1, 2, 3, 4, 5, 6, 7, 8]
RightToLeft → "xxx█▇▆▅▄▃▂▁ "
//              data[8]←→data[0] (right edge)
```

With VecDeque storing `[oldest(0), ..., newest(N)]` passed directly:
- `data[0]` (oldest value) displayed at the right edge (newest position)
- `data[N]` (newest value) displayed on the left (oldest position)
- `max_index = min(width, data.len())` limitation means only the **oldest ~80 entries** out of 300 are rendered, and newest data is never displayed

```
VecDeque: [T0(oldest), T1, T2, ..., T299(newest)]
Sparkline: data[0..80] = [T0, T1, ..., T79]  ← oldest 80 only!
Screen:     T79 ← T78 ← ... ← T1 ← T0(right edge)
```

**Bug 2: f32 truncation**

```rust
// Before: truncation (99.7% → 99)
buf.extend(src.iter().map(|&v| v as u64));
```

**Bug 3: PCIe title ambiguity**

Title `PCIe TX:12.3 RX:56.7 MB/s` but sparkline only uses `history.pcie_tx_kbps` — RX value changes not reflected in graph.

##### Fix Details (6 changes)

**Fix 1: Sparkline data reverse iteration** (`dashboard.rs`)

```rust
// Before: oldest → newest order (data[0]=oldest → right edge)
buf.extend(src.iter().map(|&v| v as u64));

// After: newest → oldest order (data[0]=newest → right edge)
buf.extend(src.iter().rev().map(|&v| v as u64));
```

Adding `.rev()` reverses data order:
- `data[0]` = newest → right edge ✓
- Only newest data within terminal width (~80) displayed ✓
- When buffer partially full, data fills from right ✓

**Fix 2: f32 rounding** (`dashboard.rs`)

```rust
// Before: truncation
buf.extend(src.iter().rev().map(|&v| v as u64));
// After: rounding
buf.extend(src.iter().rev().map(|&v| v.round() as u64));
```

**Fix 3: PCIe title clarification** (`dashboard.rs`)

```rust
// Before: "PCIe TX:12.3 RX:56.7 MB/s" — suggests graph shows TX+RX
// After: "PCIe TX:12.3 / RX:56.7 MB/s" + default title "PCIe TX"
```

**Fix 4: `GpuProcessInfo::name` → `Rc<str>`** (`metrics.rs`, `nvml.rs`)

```rust
// Before: String::clone() per tick (heap copy)
pub name: String,
fn process_name(&self, pid: u32) -> String { cache.get(&pid).clone() }

// After: Rc<str> clone = refcount bump only (zero heap allocation)
pub name: Rc<str>,
fn process_name(&self, pid: u32) -> Rc<str> { cache.get(&pid).clone() }
```

**Fix 5: `throttle_reasons` → `Cow<'static, str>`** (`metrics.rs`, `nvml.rs`)

```rust
// Before: String allocation every tick (including frequent "None")
pub throttle_reasons: Option<String>,
fn format_throttle_reasons(tr) -> String { String::from("None") }

// After: single-flag fast path → Cow::Borrowed (zero allocation)
pub throttle_reasons: Option<Cow<'static, str>>,
fn format_throttle_reasons(tr) -> Cow<'static, str> {
    // "None", "Idle", "SwPwrCap", "HW-Slow", "SW-Therm", "HW-Therm" → Borrowed
    // Only compound flags use Cow::Owned allocation
}
```

**Fix 6: `unused import: Text` warning cleanup** (`dashboard.rs`)

##### Modified Files

| File | Changes |
|------|---------|
| `src/ui/dashboard.rs` | Sparkline data `.rev()` reverse iteration, f32 rounding, PCIe title clarification, unused import cleanup |
| `src/gpu/metrics.rs` | `GpuProcessInfo::name` → `Rc<str>`, `throttle_reasons` → `Cow<'static, str>` |
| `src/gpu/nvml.rs` | `proc_name_cache` → `HashMap<u32, Rc<str>>`, `process_name()` → `Rc<str>` return, `format_throttle_reasons()` → `Cow<'static, str>` return + single-flag fast path |

##### Cross-Verification Matrix

| Scenario | Sparkline Display | Performance Impact | Verification |
|----------|------------------|-------------------|-------------|
| 300 history entries, 80-wide terminal | Newest 80 shown (newest → right) | No change | `.rev()` + `RightToLeft` |
| 10 history entries, 80-wide terminal | 10 entries filled from right edge | No change | `RightToLeft` behavior preserved |
| CPU 99.7% | Sparkline shows 100 | No change | `.round()` applied |
| PCIe TX-only graph | Title shows "PCIe TX:" explicitly | No change | Ambiguity removed |
| throttle "None" (90%+ frequency) | Same display | **String alloc eliminated** | `Cow::Borrowed` |
| throttle "SwPwrCap, HW-Therm" | Same display | Same as before (Cow::Owned) | Compound flag fallback |
| Process name cache hit | Same display | **String clone → Rc bump** | 5 per GPU × tick |
| top_processes carry-forward | Same display | **Vec<GpuProcessInfo> clone cost reduced** | Rc<str> name copies cost 0 |

##### v0.3.4 → v0.3.5 Defense Layer Relationship

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3.4: resource audit optimizations | proc_name_cache shrink + datetime cache | `nvml.rs`, `dashboard.rs` |
| **v0.3.5: Sparkline direction fix** | **Critical bug: oldest data displayed as newest → fixed** | `dashboard.rs` all sparklines |
| **v0.3.5: Process name Rc<str>** | **Eliminates per-tick String heap copy → refcount bump only** | `metrics.rs`, `nvml.rs` |
| **v0.3.5: throttle Cow<'static, str>** | **Eliminates heap alloc for frequent single-flag values** | `metrics.rs`, `nvml.rs` |

v0.3.4 provides "long-running resource reclaim optimization"; v0.3.5 provides "real-time display accuracy fix + per-tick repeated heap allocation elimination".

#### VRAM Real-Time Update Failure + Long-Running Resource Optimization (v0.3.6)

##### Symptoms

- GPU usage decreased but VRAM display failed to reflect changes in real-time
- Top Processes panel showed stale VRAM values of already-terminated processes indefinitely
- When `memory_info()` intermittently failed, previous high VRAM values were carried forward permanently

##### Root Cause Analysis — 3 Cascading Bugs

**Bug 1: Indefinite process carry-forward (most critical)**

```
Location: app.rs update_metrics()
```

When GPU usage decreased and processes terminated normally, `running_compute_processes()` returned an empty list. However, the code treated "empty list = NVML query flicker" and carried forward the previous tick's stale process list **indefinitely with no expiration check**. Dead processes' VRAM values displayed permanently until new processes appeared.

**Bug 2: Indefinite VRAM carry-forward**

```
Location: app.rs update_metrics()
```

In MIG environments, `device.memory_info()` can intermittently fail due to GPM state corruption. On failure, `memory_used = None` → previous tick's high VRAM value copied with no expiration. Even if VRAM dropped from 10GB to 2GB, failure ticks kept showing 10GB.

**Bug 3: Sparkline history double-reinforcement**

```
Location: metrics.rs push_or_repeat()
```

When `memory_used` was `None`, the last high value was repeated in history, causing the sparkline to maintain a flat high line instead of reflecting VRAM decrease.

##### Fix Details (6 changes)

**Change 1: Process carry-forward → PID liveness check**

```rust
// Before: unconditionally copy previous tick's processes (indefinite)
if m.top_processes.is_empty() {
    m.top_processes = prev.top_processes.clone();
}

// After: only carry forward processes whose PIDs are still alive via /proc/{pid}
let alive: Vec<_> = prev.top_processes.iter()
    .filter(|p| {
        buf.clear();
        write!(buf, "/proc/{}", p.pid);
        Path::new(buf.as_str()).exists()
    })
    .cloned().collect();
```

- Reflects process termination immediately — no arbitrary TTL value
- `/proc/{pid}` stat syscall is kernel-buffered at ~1μs

**Change 2: VRAM carry-forward → TTL limit of 3**

```rust
// Before: unconditionally copy previous value on memory_info() failure (indefinite)
// After: carry forward up to 3 consecutive failures, then None → "N/A"
const VRAM_CARRY_FORWARD_TTL: u32 = 3;

let count = if let Some(c) = self.vram_fail_count.get_mut(&m.uuid) {
    *c += 1; *c
} else {
    self.vram_fail_count.insert(m.uuid.clone(), 1); 1
};
if count <= VRAM_CARRY_FORWARD_TTL { /* carry forward */ }
// else: UI shows "N/A"
```

- Default 1s interval × 3 = 3s tolerance (covers transient flicker)
- Counter resets immediately on success

**Change 3: /proc/{pid} path buffer reuse**

```rust
// Before: format!("/proc/{}", pid) per PID → String heap allocation
// After: reuse proc_path_buf: String in App struct
buf.clear();
write!(buf, "/proc/{}", p.pid);  // reuses existing buffer, 0 allocations
```

- Eliminates 25+ String allocations per tick (5 GPUs × 5 processes)
- Prevents ~2.7 billion unnecessary allocations over 300 hours

**Change 4: active_handles Vec → HashSet**

```rust
// Before: Vec<usize> → cache.retain(|k, _| active_handles.contains(k))  // O(n) per entry
// After: HashSet<usize> → contains() O(1)
active_handles: RefCell<HashSet<usize>>,
```

- `prune_stale_caches()` complexity: O(n²) → O(n)
- With 128 MIG instances (16 GPUs × 8): 16,384 comparisons → 128 hash lookups

**Change 5: History cleanup UUID HashSet pre-build**

```rust
// Before: self.history.keys().any(|uuid| !new_metrics.iter().any(...))  // O(n×m)
// After: pre-build HashSet → O(1) lookup
let uuid_set: HashSet<&Rc<str>> = new_metrics.iter().map(|m| &m.uuid).collect();
self.history.retain(|uuid, _| uuid_set.contains(uuid));
```

- Double-nested `.any()` O(n×m) → HashSet O(n) single pass

**Change 6: vram_fail_count entry() Rc clone avoidance**

```rust
// Before: self.vram_fail_count.entry(m.uuid.clone()).or_insert(0)  // Rc clone every time
// After: get_mut/insert pattern → 0 clones on cache hit
let count = if let Some(c) = self.vram_fail_count.get_mut(&m.uuid) {
    *c += 1; *c
} else {
    self.vram_fail_count.insert(m.uuid.clone(), 1); 1
};
```

##### Modified Files

| File | Changes | Purpose |
|------|---------|---------|
| `app.rs` | PID liveness check + VRAM TTL + proc_path_buf + UUID HashSet + Rc clone avoidance | VRAM real-time update + resource optimization |
| `nvml.rs` | `active_handles` Vec→HashSet + signature changes | prune O(n²)→O(n) |

##### Cross-Verification Matrix

| Scenario | Verification Target | Expected Result |
|----------|-------------------|-----------------|
| GPU usage decrease → process exit | Top Processes panel | Terminated processes disappear immediately |
| memory_info() 1-3 consecutive failures | VRAM gauge | Previous value retained (tolerance) |
| memory_info() 4+ consecutive failures | VRAM gauge | Shows "N/A" |
| memory_info() failure then success | VRAM gauge + counter | Immediately reflects actual value, counter resets |
| 100 repeated MIG reconfigs | active_handles HashSet | prune O(n), constant memory |
| 128 MIG instances | history cleanup | O(n) HashSet lookup (removes previous O(n×m)=16K comparisons) |
| 300-hour long run | proc_path_buf | 0 String allocations (buffer reuse) |
| vram_fail_count normal tick | Rc clone | get_mut hit → 0 clones |
| GPU removal | vram_fail_count | Cleaned up via retain() |

##### v0.3.5 → v0.3.6 Defense Layer Relationship

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3.5: Sparkline direction + heap optimization | sparkline accuracy + Rc/Cow optimization | `dashboard.rs`, `metrics.rs`, `nvml.rs` |
| **v0.3.6: PID liveness check** | **Immediate process carry-forward expiry** | `app.rs` process display |
| **v0.3.6: VRAM TTL** | **memory_info() failure limited to 3 attempts** | `app.rs` VRAM display |
| **v0.3.6: active_handles HashSet** | **prune O(n²)→O(n)** | `nvml.rs` cache cleanup |
| **v0.3.6: UUID HashSet** | **history cleanup O(n×m)→O(n)** | `app.rs` GPU removal detection |

v0.3.5 provides "sparkline accuracy + per-tick heap optimization"; v0.3.6 provides "VRAM real-time update fix + carry-forward safety + algorithmic complexity improvement".

#### VRAM N/A Transition + Long-Running Memory Optimization (v0.3.8)

##### Symptoms

- VRAM displays normally at first, then **permanently switches to "N/A"** after a few seconds
- Only occurs in MIG environments with GPM (Hopper+)
- 128-core systems generate ~384 String allocations per tick for CPU core bar rendering

##### Root Cause Analysis — 2 Issues

**Issue 1: GPM Cross-Tick NVML State Corruption (Critical)**

```
Location: nvml.rs collect_mig_instances() Phase 2
```

`nvmlGpmMigSampleGet()` calls corrupt NVML driver internal state, and this corruption **persists across ticks**, causing `memory_info()` to fail permanently.

```
Tick 1: Phase 1 memory_info() succeeds → Phase 2 GPM call → driver state corrupted
Tick 2: Phase 1 memory_info() fails (previous tick corruption) → carry-forward starts
Tick 4: VRAM_CARRY_FORWARD_TTL(3) exceeded → memory_used = None → "N/A" locked
```

The 2-phase design protects VRAM **within the same tick**, but does not defend against the cross-tick problem where **previous tick's Phase 2 corruption affects next tick's Phase 1**.

**Issue 2: Unnecessary Per-Tick Allocations in Long-Running Operation**

| Item | Allocation Pattern | Impact |
|------|-------------------|--------|
| MIG display name | `format!().into()` → new `Rc<str>` per tick | MIG instances × tick |
| `phase1` Vec | `Vec::new()` → realloc per tick | During MIG collection |
| `seen_pids` HashSet | `HashSet::new()` → per-device alloc per tick | Devices × tick |
| CPU bar strings | `make_bar()` → cores × `String` alloc | 128 cores = 128 alloc/draw |
| Time string | `TIME_CACHE.clone()` + `format!()` | 2 alloc/draw |

##### Fix Details (8 changes)

**Change 1: Complete GPM MIG Phase 2 Removal** (`nvml.rs`)

```rust
// Removed: Entire Phase 2 GPM fallback block
// nvmlGpmMigSampleGet() calls eliminated → NVML state corruption prevented at source
// memory_util shown only when available via Fallback 1 (process utilization)
```

Losing a single `memory_util` (DRAM BW) metric does not justify losing the critical VRAM metric, so GPM Phase 2 is completely skipped in MIG environments.

**Change 2: MIG Display Name Caching** (`nvml.rs`)

```rust
// Before: new Rc<str> every tick
metrics.name = format!("MIG {mig_idx} (GPU {gpu_index}: {})", metrics.name).into();
// After: cached in DeviceInfo.mig_display_name, Rc::clone() only
let cached = device_cache.get(&key).and_then(|i| i.mig_display_name.clone());
metrics.name = cached.unwrap_or_else(|| { /* format + cache + return */ });
```

**Change 3: `phase1` Vec Pre-allocation** (`nvml.rs`)

```rust
// Before: Vec::new() → possible realloc on each push
// After: Vec::with_capacity(max_count) → single allocation
```

**Change 4: PID Dedup HashSet Reuse** (`nvml.rs`)

```rust
// Before: HashSet::new() per device per tick
// After: NvmlCollector.proc_seen_pids: RefCell<HashSet<u32>> reused
//        + parent_procs/entries Vec::with_capacity(16)
```

**Change 5: Time String Zero-Alloc Rendering** (`dashboard.rs`)

```rust
// Before: c.1.clone() from TIME_CACHE + format!(" {} ", now) → 2 alloc/draw
// After: rendering moved inside TIME_CACHE.with closure
//        + write!(c.1, ...) buffer reuse + c.1.as_str() reference → 0 alloc/draw
```

**Change 6: CPU Bar Lookup Table** (`dashboard.rs`)

```rust
// Before: make_bar(usage, bar_width) → String alloc per core (128 cores = 128 alloc)
// After: BAR_TABLE thread_local! lookup table
//        bar_width+1 patterns pre-built, bt.1[filled].as_str() reference
//        rebuilds only on terminal resize → 0 alloc/draw (after first draw)
```

**Change 7: thread_local const Initialization** (`dashboard.rs`)

```rust
// clippy recommendation: use const for thread_local! initializers
static TIME_CACHE: ... = const { RefCell::new(...) };
static BAR_TABLE: ... = const { RefCell::new(...) };
```

**Change 8: entries/parent_procs Pre-allocation** (`nvml.rs`)

```rust
// Before: Vec::new() → alloc on first push
// After: Vec::with_capacity(16) → initial allocation matching expected process count
```

##### Modified Files

| File | Changes | Related |
|------|---------|---------|
| `src/gpu/nvml.rs` | GPM Phase 2 removal + MIG name caching + Vec/HashSet reuse | Changes 1, 2, 3, 4, 8 |
| `src/ui/dashboard.rs` | Time string zero-alloc + BAR_TABLE lookup + const init | Changes 5, 6, 7 |

##### Cross-Verification Matrix

| Verification Item | Method | Expected Result |
|-------------------|--------|-----------------|
| VRAM N/A transition | Long-running on MIG + Hopper | VRAM values maintained, no N/A transition |
| memory_util display | When Fallback 1 (process util) available | Normal display (N/A when unavailable) |
| MIG name caching | Run 2+ ticks, check DeviceInfo cache | Format only on first tick, Rc::clone thereafter |
| CPU bar allocation | 128-core system draw | Zero bar String allocations after first draw |
| Time string | draw_header per-frame | No clone/format allocations |
| cargo clippy | Check warnings | No new warnings |

##### v0.3.6 → v0.3.8 Defense Layer Relationship

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3.6: VRAM TTL (3 attempts) | carry-forward limit on memory_info() failure | `app.rs` VRAM display |
| **v0.3.8: GPM MIG disabled** | **cross-tick NVML state corruption prevented at source** | `nvml.rs` MIG collection |
| **v0.3.8: MIG name caching** | **per-tick Rc<str> allocation eliminated** | `nvml.rs` DeviceInfo cache |
| **v0.3.8: PID dedup reuse** | **per-tick HashSet allocation eliminated** | `nvml.rs` process collection |
| **v0.3.8: BAR_TABLE lookup** | **per-draw 128+ String allocations eliminated** | `dashboard.rs` CPU core bars |
| **v0.3.8: time string zero-alloc** | **per-draw 2 String allocations eliminated** | `dashboard.rs` header |

v0.3.6 provides "VRAM TTL to prevent stagnation"; v0.3.8 provides "GPM corruption prevention at source + long-running per-tick allocation minimization".

#### RAM Segmented Chart Visual Gap Fix (v0.3.9)

##### Symptom

- In the RAM segmented chart, green (used) and blue (cached) areas appear **visually separated**
- When green proportion decreases, blue does not stay attached to green — a gap appears between the two segments
- Particularly noticeable when used ratio is low (< 20%)

##### Root Cause Analysis

```
Location: dashboard.rs draw_ram_segmented_chart()
```

When used (green) ends with a fractional block character (`▁▂▃▄▅▆▇`), the character only fills the **bottom portion** of the cell. The remaining upper portion of the cell is left as background color (black). Cached (blue) starts from the **next cell** with `█` (full block), so the empty upper portion of the green fractional cell creates a visible gap.

```
Before (gap present):
│█│ ← cached (blue, full block)
│▃│ ← used (green, 3/8 block) — upper 5/8 is empty background
│█│ ← used (green, full block)

After (seamless):
│█│ ← cached (blue, full block)
│▃│ ← used (green fg, blue bg) — lower 3/8 green, upper 5/8 blue
│█│ ← used (green, full block)
```

##### Fix Details (1 change)

**Change 1: Apply cached background color to fractional used cell** (`dashboard.rs`)

```rust
// Before: only fg set, bg defaults to black
} else if bottom_row == used_rows && used_frac > 0.05 {
    (bar_chars[(used_frac * 8.0) as usize % 8], used_color)

// After: fg=used_color, bg=Color::Blue (when cached exists)
} else if bottom_row == used_rows && used_frac > 0.05 {
    let bg = if has_cached { Color::Blue } else { Color::Reset };
    (bar_chars[(used_frac * 8.0) as usize % 8], used_color, bg)
```

Within a single cell, bottom (fg) = green used and top (bg) = blue cached are rendered simultaneously, ensuring visual continuity.

##### Modified Files

| File | Change | Related |
|------|--------|---------|
| `src/ui/dashboard.rs` | Fractional used cell bg color + 3-tuple return structure | Change 1 |

##### Cross-Verification Matrix

| Verification Item | Method | Expected Result |
|-------------------|--------|----------------|
| used+cached seamless | used 10-30% + cached 50%+ scenario | No gap between green and blue |
| used 0% (cached only) | Only cached present | Blue rendered from bottom |
| cached 0% (used only) | Only used present | Green from bottom, bg=Reset |
| 100% usage | used + cached = 100% | Chart fully filled, no empty space |
| cargo clippy | Check warnings | No new warnings |

##### v0.3.8 → v0.3.9 Defense Layer Relationship

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3.8: GPM MIG disabled + per-tick alloc minimization | cross-tick corruption prevention + long-running memory | `nvml.rs`, `dashboard.rs` |
| **v0.3.9: RAM chart fractional cell bg color** | **Eliminates visual gap at used-cached boundary** | `dashboard.rs` RAM segmented chart |

v0.3.8 provides "data collection stability + allocation optimization"; v0.3.9 provides "RAM chart visual accuracy fix".

#### MIG Mem Ctrl GPM Restoration + Per-Tick Redundancy Elimination (v0.3.10)

##### Symptom

- MIG instances always show **"N/A"** for Mem Ctrl. (Memory Controller Utilization)
- Before v0.3.8, GPM provided values with a 1-2 second delay
- GPU Util and VRAM work normally, but Mem Ctrl. has no collection path

##### Root Cause Analysis

```
Location: nvml.rs collect_mig_instances()
```

In v0.3.8, GPM (`nvmlGpmMigSampleGet`) was found to corrupt NVML state across ticks, causing the VRAM "N/A" issue. The entire GPM Phase 2 was **completely removed**. However, this eliminated all collection paths for MIG memory_util:

```
Collection path analysis (v0.3.8~v0.3.9):
1. utilization_rates()       → Always NVML_ERROR_NOT_SUPPORTED on MIG
2. GPM fallback (collect_device_metrics) → Blocked by !is_mig condition
3. GPM fallback (collect_mig_instances) → Removed in v0.3.8
4. process utilization       → None when no processes or mem=0
→ Result: No memory_util collection path → always "N/A"
```

**Key insight**: GPM corrupts NVML state, but `memory_info()` is **already collected** in Phase 1. Calling GPM after Phase 1 keeps VRAM data safe.

##### Fix Details (4 changes)

**Change 1: Phase 1.5 GPM DRAM BW Util Restoration** (`nvml.rs`)

```rust
// Phase 1: All MIG instances' VRAM (memory_info()) collection complete
// Phase 1.5 (new): Collect memory_util via GPM — VRAM is already safe
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
// Phase 2: Process collection (existing)
```

Execution order Phase 1 → 1.5 → 2 ensures `memory_info()` is already complete when GPM is called.
First tick returns `None` (needs previous sample for delta calculation); values appear from second tick onward.

**Change 2: `gpu_instance_id` Phase 1 Caching to Eliminate Redundant Device::new** (`nvml.rs`)

```rust
// Before: Device::new() + get_device_info() in Phase 1, 1.5, 2 = 3×/instance
// phase1: Vec<(nvmlDevice_t, GpuMetrics)>
let mig_device = Device::new(*mig_handle, &self.nvml);     // Phase 1.5
let mig_info = self.get_device_info(&mig_device, true);    // Phase 1.5
let mig_device = Device::new(*mig_handle, &self.nvml);     // Phase 2
let mig_info = self.get_device_info(&mig_device, true);    // Phase 2

// After: Cache gi_id in Phase 1, use directly in 1.5/2 = 1×/instance
// phase1: Vec<(nvmlDevice_t, Option<u32>, GpuMetrics)>
let mig_info = self.get_device_info(&mig_device, true);    // Once in Phase 1
let gi_id = mig_info.gpu_instance_id;                      // Cached
// Phase 1.5/2 use gi_id directly — no Device::new/get_device_info needed
```

For 7 MIG instances: 14 HashMap lookups/tick → 0.

**Change 3: Eliminate Redundant `get_device_info` in Fallback 2** (`nvml.rs`)

```rust
// Before: get_device_info(&mig_device, true) called again in Fallback 2
if metrics.gpu_util.is_none() {
    let mig_info = self.get_device_info(&mig_device, true);  // redundant!
    if let Some(mig_slices) = mig_info.gpu_instance_slice_count { ... }
}

// After: Reuse mig_info already fetched at top of Phase 1 loop
if metrics.gpu_util.is_none() {
    if let Some(mig_slices) = mig_info.gpu_instance_slice_count { ... }
}
```

**Change 4: Eliminate Double HashSet Allocation in app.rs** (`app.rs`)

```rust
// Before: HashSet created twice — once for condition check, once for retain
if self.history.len() != new_metrics.len()
    || {
        let uuid_set: HashSet<_> = new_metrics.iter().map(|m| &m.uuid).collect();  // 1st
        self.history.keys().any(|uuid| !uuid_set.contains(uuid))
    }
{
    let uuid_set: HashSet<_> = new_metrics.iter().map(|m| &m.uuid).collect();  // 2nd (same)
    self.history.retain(|uuid, _| uuid_set.contains(uuid));
}

// After: Single allocation used for both condition check and retain
let uuid_set: HashSet<_> = new_metrics.iter().map(|m| &m.uuid).collect();  // once
if self.history.len() != uuid_set.len()
    || self.history.keys().any(|uuid| !uuid_set.contains(uuid))
{
    self.history.retain(|uuid, _| uuid_set.contains(uuid));
}
```

##### Modified Files

| File | Changes | Related |
|------|---------|---------|
| `src/gpu/nvml.rs` | Phase 1.5 GPM restoration + gi_id caching + Fallback 2 dedup | Changes 1, 2, 3 |
| `src/app.rs` | HashSet double allocation elimination | Change 4 |

##### Cross-Verification Matrix

| Verification Item | Method | Expected Result |
|-------------------|--------|-----------------|
| Mem Ctrl. display | Run 2+ ticks on MIG + Hopper | First tick "N/A" → 0-100% from second tick |
| VRAM safety | Long-running execution | GPM called after Phase 1 → no VRAM "N/A" transition |
| GPU Util normal | Fallback 1/2 paths | Existing behavior maintained |
| gi_id caching | No Device::new in Phase 1.5/2 | 0 HashMap lookups (cached gi_id used directly) |
| HashSet single alloc | GPU reconfig scenario | Same HashSet created only once |
| Pre-Ampere GPUs | GPM not supported | Mem Ctrl. stays "N/A" (gpm_supported=false → skipped) |
| cargo clippy | Check warnings | No new warnings |

##### Resource Leak Audit Results

| Resource | Protection Mechanism | Status |
|----------|---------------------|--------|
| MetricsHistory VecDeque | `push_ring()` + max_entries cap | ✓ bounded |
| device_cache HashMap | `prune_stale_caches()` every tick | ✓ pruned |
| proc_name_cache | Per-tick retain active PIDs only + shrink_to | ✓ pruned |
| gpm_prev_samples | `nvmlGpmSampleFree` on stale entry prune | ✓ freed |
| proc_sample_buf / sample_buf | Grow-only BUT shrink when `capacity > floor*8` | ✓ bounded |
| App history/vram_fail_count | UUID-based retain + shrink_to | ✓ pruned |

All caches/buffers properly pruned; no unbounded growth confirmed for long-running operation.

##### v0.3.9 → v0.3.10 Defense Layer Relationship

| Defense Layer | Protection Scope | Applied At |
|--------------|-----------------|-----------|
| v0.3.9: RAM chart fractional cell bg color | Eliminates visual gap at used-cached boundary | `dashboard.rs` RAM segmented chart |
| **v0.3.10: Phase 1.5 GPM restoration** | **Safe GPM call after Phase 1 restores Mem Ctrl. collection** | `nvml.rs` MIG collection Phase 1.5 |
| **v0.3.10: gi_id pre-caching** | **Eliminates per-tick Device::new + get_device_info redundancy** | `nvml.rs` Phase 1 → 1.5/2 |
| **v0.3.10: HashSet single allocation** | **Eliminates double HashSet creation on GPU reconfig** | `app.rs` history cleanup |

v0.3.9 provides "RAM chart visual accuracy fix"; v0.3.10 provides "Mem Ctrl. GPM safe restoration + per-tick redundancy elimination".

### NVML API Latency Benchmark

Measured on H100 PCIe, 1000 iterations per API call:

| API Call | Avg Latency | Notes |
|---|---|---|
| `nvmlDeviceGetSamples(GPU_UTIL)` 2-phase | **835 µs** | New addition for MIG fallback |
| `nvmlDeviceGetUtilizationRates()` (fails) | 202 µs | Fast even on failure path |
| `temperature()` | 234 µs | Simple sensor read |
| `power_usage()` | 3,592 µs | Most expensive (hardware SMBus) |
| `clock_info(Graphics)` | 1,489 µs | Moderate |
| `memory_info()` | 7 µs | Fastest |

The new `nvmlDeviceGetSamples` call adds ~835µs per tick — less than 0.1% overhead at 1-second intervals, and cheaper than the existing `power_usage()` call.

Buffer: 128 entries × 16 bytes = 2,048 bytes (reused via `RefCell<Vec>`, no per-tick allocation).

## Performance Optimization

Resource usage is minimized so that the monitoring tool itself does not affect GPU workloads.

### Expected Resource Consumption

Estimates for default settings (1-second interval), 1 GPU + 2 MIG instances:

| Resource | Expected Usage | Notes |
|----------|---------------|-------|
| **CPU** | **0.5~2.5% (per core)** | ~7-20ms active time per tick, ~980-993ms sleep |
| **RSS Memory** | **4~8 MB** | Binary + libnvidia-ml.so + history buffers + TUI buffers |
| **GPU Compute** | **0% (unused)** | NVML is read-only driver IPC, no CUDA context created |
| **GPU VRAM** | **0 MB (unused)** | No GPU memory allocation |
| **Disk I/O** | **Effectively 0** | Only reads `/proc` (virtual procfs) — no actual disk access |
| **Network** | **0** | No network communication |

#### Per-Tick Time Breakdown (1-second interval)

```
1 tick = 1000ms
├── NVML API calls        ~7-18ms   Driver IPC (15-19 queries per GPU)
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
│   ├── running_compute/graphics_procs ~0.5ms  compute+graphics merged + /proc name reads for top-5 only
│   └── (MIG) process_util     ~1-3ms   Per MIG instance, fallback only
│   └── (MIG) gpu_util_samples ~0.8ms   nvmlDeviceGetSamples fallback, parent only
├── sysinfo refresh       ~0.1-0.3ms
│   ├── refresh_cpu_usage      ~0.1ms   Reads /proc/stat
│   └── refresh_memory         ~0.05ms  Reads /proc/meminfo
├── TUI rendering         ~0.5-2ms   ratatui diff buffer + ANSI output
├── Event wait (sleep)    ~980-993ms  crossterm poll, kernel scheduling
└── Total active time     ~7-20ms    = CPU 0.7-2.0%
```

#### RSS Memory Breakdown

```
Total RSS ~4-8 MB
├── Binary code/data segments          ~1.4 MB   (mmap)
├── libnvidia-ml.so shared library     ~2-4 MB   (mmap, shared with system)
├── History ring buffers               ~80 KB
│   ├── MetricsHistory per GPU          ~22 KB   (9 VecDeque × 300 × 4-8B)
│   │   (× 3 devices = ~42 KB)
│   └── SystemHistory                   ~7 KB    (4 VecDeque × 300 × 4-8B, incl. ram_used_pct/ram_cached_pct)
├── ratatui Terminal double buffer     ~50-400 KB (proportional to terminal size)
│   (80×24: ~77KB, 200×50: ~400KB)
├── sysinfo System struct              ~30-50 KB  (CPU only, no processes)
├── Reusable buffers                    ~5 KB
│   ├── thread_local sparkline buf      ~2.4 KB
│   ├── proc_sample_buf                 ~1 KB
│   ├── gpu_sample_buf                    ~2 KB
│   └── cpu_buf                         ~0.3 KB
└── HashMap, String caches, etc.        ~5-10 KB
```

#### Resource Comparison by Interval

| Interval | CPU Usage | Characteristics |
|----------|-----------|-----------------|
| `500ms` | ~1.5-4% | Fast response, slightly increased monitoring overhead |
| `1000ms` (default) | ~0.7-2.0% | Balanced default |
| `2000ms` | ~0.4-1.0% | Resource saving, for large-scale clusters |
| `5000ms` | ~0.1-0.4% | Minimum overhead, for long-term observation |

> RSS memory is the same regardless of interval. Since the history entry count (300) is fixed,
> longer intervals record a longer time range (1s: 5min, 2s: 10min, 5s: 25min).

### Optimization Details: Memory

| Optimization | Location | Before → After |
|-------------|----------|----------------|
| `VecDeque` ring buffer | `metrics.rs` | `Vec::remove(0)` O(n) memmove → `VecDeque::pop_front()` O(1) |
| Device info cache | `nvml.rs` | NVML API + String alloc every tick → `RefCell<HashMap>` first call only, cache hit thereafter |
| Process sample buffer | `nvml.rs` | `vec![zeroed(); N]` alloc/dealloc per MIG call → `RefCell<Vec>` grow-only reuse |
| CPU buffer zero-copy swap | `main.rs` | `Vec::clone()` every tick → `std::mem::take` + reclaim buffer from previous SystemMetrics (zero alloc after first tick) |
| Sparkline conversion buffer | `dashboard.rs` | 5× `Vec<u64>` alloc per draw → `thread_local!` single scratch reuse |
| Process partial sort | `nvml.rs` | O(n log n) full sort → O(n) `select_nth_unstable_by` (when > 5 processes) |
| Deferred process name I/O | `nvml.rs` | Read `/proc/{pid}/comm` for all N processes → select top-5 first, read names for only 5 (up to 95% I/O reduction) |
| CPU cores Vec reuse | `dashboard.rs` | Vec alloc per draw → `thread_local!` buffer reuse |
| `make_bar()` string | `dashboard.rs` | `.repeat()` 2× concatenation → `String::with_capacity` + push loop |
| HashMap uuid clone | `app.rs` | `uuid.clone()` every tick → `contains_key` then clone only on miss |
| GPU history auto-cleanup | `app.rs` | Unbounded HashMap growth on MIG reconfig/GPU removal → `retain()` removes orphan UUID entries |
| GPM sample + device cache auto-pruning | `nvml.rs` | Stale `nvmlGpmSample_t` + `DeviceInfo` leaked on MIG reconfig → per-tick active handle tracking + `retain()` + `nvmlGpmSampleFree()` |
| NVML sample buffer shrink | `nvml.rs` | grow-only buffer could grow unbounded → auto `shrink_to(needed×2)` when capacity > needed×2 |
| Sparkline RightToLeft direction | `dashboard.rs` | All 5 sparklines use `RenderDirection::RightToLeft` → unified right-to-left progression matching RAM segmented chart |
| RAM chart zero-alloc rendering | `dashboard.rs` | Per-frame `Vec<ColSegment>` allocation → direct iterator + buffer write (zero allocation) |
| RAM segmented chart stacking fix | `dashboard.rs` | Used fractional row shifted cached start point, causing cached to lose ~1 row per column → introduced `cached_base` to correctly offset cached stacking based on used fraction presence |
| RAM chart fractional cell bg color | `dashboard.rs` | Used fractional character (`▃▄` etc.) left upper cell portion as background → `cell.set_bg(Color::Blue)` applied, bottom=used(fg) top=cached(bg) seamless display within single cell |
| RAM calculation accuracy fix | `dashboard.rs` | `used = ram_used - (avail-free)` (double subtraction) → `used = total - available` (correct non-reclaimable memory) |
| `format_pstate` zero-alloc | `nvml.rs` | `"P0".to_string()` per tick → returns `&'static str` (zero allocation) |
| `format_architecture` zero-alloc | `nvml.rs` | Same pattern: `"Ampere".to_string()` → `&'static str` |
| `format_throttle_reasons` Vec removal | `nvml.rs` | `Vec::new()` + `push` + `join()` → macro appends directly to `String` (eliminates Vec allocation) |
| `GIB_F64` module constant | `metrics.rs` | Redundant `1024.0 * 1024.0 * 1024.0` computation → single `const GIB_F64` definition, reused globally |
| `ram_breakdown()` unified calc | `metrics.rs` | Duplicate RAM decomposition in `draw_ram_swap` + `draw_memory_legend` → single `SystemMetrics::ram_breakdown()` call |
| `active_handles` Vec reuse | `nvml.rs` | Per-tick `Vec::with_capacity(N)` alloc/dealloc → `RefCell<Vec<usize>>` field reuse (zero alloc per tick) |
| Sparkline title `Cow<str>` | `dashboard.rs` | Static strings ("N/A", fallback) `to_string()` allocation → `Cow::Borrowed` zero-alloc, `format!` only for dynamic values |
| Top Processes header static `&str` | `dashboard.rs` | 3× `format!()` calls per frame for header → static `&str` Spans (eliminates 3 String allocations per frame) |
| Top Processes column alignment fix | `dashboard.rs` | Header (hardcoded 8+22+4) vs data (`{:<7}`+`{:<15}`+`{:>10}`) width mismatch → unified format widths |
| Compute + graphics process unification | `nvml.rs` | Compute-only collection → both compute + graphics collected + HashSet PID dedup, Unavailable VRAM processes preserved |
| `GpuProcessInfo.vram_used` Option | `metrics.rs` | `u64` → `Option<u64>`, VRAM Unavailable processes display "N/A" (previously: filtered out entirely) |
| `truncate_str()` zero-alloc | `dashboard.rs` | `proc.name.chars().take(15).collect::<String>()` 5 allocs/frame → `&str` slicing (zero allocation) |
| `Rc<str>` string sharing | `nvml.rs`, `metrics.rs`, `app.rs` | `DeviceInfo`/`GpuMetrics` name·uuid·compute_capability changed to `Rc<str>` → eliminates heap allocation on clone (reference count bump only) |
| `ram_breakdown()` single call | `dashboard.rs` | Duplicate calculation in `draw_ram_bars` + `draw_memory_legend` → computed once in `draw_system_charts`, passed to both |
| Process name caching | `nvml.rs` | Per-tick `/proc/{pid}/comm` I/O → `HashMap<u32, String>` cache + automatic dead PID cleanup each tick |
| NVML buffer shrink threshold | `nvml.rs` | `capacity > needed*2` → `capacity > floor*8` threshold, prevents shrink↔resize thrashing on oscillating process/sample counts |
| `device_cache` HashMap defensive shrink | `nvml.rs` | Prevents unbounded HashMap capacity growth on repeated MIG reconfigs → auto-shrink when `capacity > len*4` |
| `gpm_prev_samples` defensive shrink | `nvml.rs` | Same shrink heuristic applied to GPM sample HashMap → auto-shrink when `capacity > len*4`, reclaims memory on repeated MIG reconfigs |
| proc_name_cache HashSet-based pruning | `nvml.rs` | Per-tick dead PID pruning changed from O(n·m) nested iteration → `HashSet<u32>` O(n+m) lookup, consistent performance as process count grows |
| Memory panel consolidated to right | `dashboard.rs` | Removed left Memory box → RAM/SWP bars integrated into right System Charts, expanding CPU core display area |
| MIG process parent device fallback | `nvml.rs` | When MIG handle process API fails, query parent GPU → filter by `gpu_instance_id` to distribute processes per MIG instance (Phase 3) |
| Top Processes carry-forward | `app.rs` | Retains previous tick's process list when NVML process API intermittently fails → prevents flicker |
| Phase 3 gi_id fallback relaxation | `nvml.rs` | Shows all parent processes when `gpu_instance_id` unavailable (previously: all filtered out) |
| datetime format cache | `dashboard.rs` | Per-tick `chrono::format().to_string()` → `thread_local!` cache, re-format only when second changes |
| GPU history HashMap accurate pruning | `app.rs` | Removed `len > metrics.len()` condition → always prune on UUID mismatch + added `shrink_to()` (prevents orphans on MIG reconfig) |
| proc_name_cache shrink threshold improvement | `nvml.rs` | `len.max(16) * 4` → `target * 2` threshold, more aggressive memory reclaim on mass PID death |
| Sparkline data direction fix | `dashboard.rs` | `data[0]=oldest` displayed at right edge bug → `.rev()` converts to `data[0]=newest`, newest data correctly at right edge |
| f32→u64 rounding | `dashboard.rs` | `v as u64` truncation (99.7→99) → `v.round() as u64` (99.7→100), CPU sparkline precision improvement |
| `GpuProcessInfo::name` → `Rc<str>` | `metrics.rs`, `nvml.rs` | Per-tick `String::clone()` heap copy → `Rc::clone()` refcount bump only (5 processes × GPU × tick heap alloc eliminated) |
| `throttle_reasons` → `Cow<'static, str>` | `metrics.rs`, `nvml.rs` | Per-tick `String` heap alloc → "None", "Idle" etc. single flags use `Cow::Borrowed` zero-alloc (covers 90%+ of real usage) |
| `proc_name_cache` → `HashMap<u32, Rc<str>>` | `nvml.rs` | Cache hit `String::clone()` → `Rc::clone()` refcount bump only, process name sharing cost eliminated |
| PCIe sparkline title clarification | `dashboard.rs` | Title "TX/RX" label mismatched graph content (TX only) → clarified to "PCIe TX:N / RX:N MB/s" |
| Sparkline carry-forward TTL | `metrics.rs` | Indefinite last-value repeat on None → `none_counts[9]` per-metric array with 3-tick TTL, stops pushing after expiry (prevents stale sparklines) |
| `get_process_utilization` Option return | `nvml.rs` | API failure returned `(0, 0)` → returns `Option<(u32, u32)>`, distinguishes idle 0% from failure, prevents false fallback scaling |
| `collect_all()` per-device error isolation | `nvml.rs` | `device_by_index(i)?` failed entire collection on single GPU error → `match ... continue` skips failed GPU, remaining GPUs collected normally |
| GPM MIG Phase 2 removal | `nvml.rs` | `nvmlGpmMigSampleGet()` calls in MIG → completely removed, prevents cross-tick NVML state corruption causing VRAM N/A |
| MIG display name caching | `nvml.rs` | Per-tick `format!().into()` new `Rc<str>` → `DeviceInfo.mig_display_name` cache, `Rc::clone()` reuse |
| PID dedup HashSet reuse | `nvml.rs` | Per-tick `HashSet::new()` per device → `proc_seen_pids: RefCell<HashSet<u32>>` reuse (zero alloc per tick) |
| `make_bar()` lookup table | `dashboard.rs` | Per-core `String` alloc → `BAR_TABLE` thread-local lookup, `&str` reference only (128 cores: 128 alloc → 0/draw) |
| Time string zero-alloc | `dashboard.rs` | `clone()` + `format!()` 2 alloc/draw → `write!` buffer reuse + `as_str()` reference (0 alloc/draw) |
| `phase1` Vec pre-allocation | `nvml.rs` | `Vec::new()` → `Vec::with_capacity(max_count)`, eliminates realloc during MIG collection |
| entries/parent_procs pre-allocation | `nvml.rs` | `Vec::new()` → `Vec::with_capacity(16)`, eliminates first-push alloc during process collection |

### Optimization Details: CPU (Minimize System Calls)

| Optimization | Location | Effect |
|-------------|----------|--------|
| `System::new()` | `main.rs` | Eliminates full process/disk/network scan vs `new_all()` |
| Targeted refresh | `main.rs` | Only `refresh_cpu_usage()` + `refresh_memory()` — reads just /proc/stat and /proc/meminfo |
| Default interval 1000ms | `main.rs` | Halves all syscall + NVML call frequency vs 500ms |
| CPU priming | `main.rs` | Prevents sysinfo's first `refresh_cpu_usage()` returning 0% — one pre-call at init |
| Drift-corrected tick loop | `main.rs` | Cumulative drift from `work_time + interval` → `Instant`-based elapsed measurement, poll only `interval - elapsed` |

### Optimization Details: GPU (Minimize NVML Calls)

| Optimization | Location | Effect |
|-------------|----------|--------|
| `utilization_rates()` first | `nvml.rs` | Try even on MIG, fallback to process util only on failure (saves 2 extra IPCs) |
| `nvmlDeviceGetSamples` fallback | `nvml.rs` | Parent-level GPU util sampling when `utilization_rates()` fails on MIG, scaled by slice ratio — buffer reused via `RefCell<Vec>` |
| Process util 2-pass | `nvml.rs` | 1st call with count=0 to get size, 2nd call to fetch data — prevents over-allocation |
| `RefCell` interior mutability | `nvml.rs` | Allows cache/buffer mutation with `&self` while NVML handles borrow, no borrow checker conflicts |
| Deferred process name reads (top-5) | `nvml.rs` | Read `/proc` for all N processes → collect pid+VRAM only, select top-5, then read `/proc/{pid}/comm` for just 5 |
| GPM + device cache per-tick pruning | `nvml.rs` | Track active handles → free stale `nvmlGpmSample_t` + remove `DeviceInfo`, prevents NVML resource leaks on MIG reconfig |
| Zero GPU resource usage | Design | NVML is read-only driver query — no CUDA context, no VRAM allocation |

### Optimization Details: Binary Size

| Setting | Value | Effect |
|---------|-------|--------|
| `opt-level` | 3 | Maximum optimization |
| `lto` | true | Link-Time Optimization, dead code elimination |
| `strip` | true | Complete debug symbol removal |
| `codegen-units` | 1 | Single codegen unit for whole-program optimization (slower build, faster runtime) |
| `panic` | "abort" | Removes unwind code — smaller binary + immediate exit on panic |
| `tokio` removal | — | No async needed, synchronous event loop suffices — saves ~200KB |
| Final size | **~1.5MB** | Single binary (dynamically links libc) |

## Runtime Stability (Long-Running Safety)

Designed for stable 24/7 operation with no memory growth or resource leaks.

### Memory Stability

| Protection Mechanism | Location | Description |
|---------------------|----------|-------------|
| VecDeque ring buffer (300 fixed) | `metrics.rs` | GPU/system history size fixed, cannot grow unbounded |
| GPU history auto-cleanup + shrink | `app.rs` | UUID mismatch detection on MIG reconfig/GPU removal → orphan entries auto-deleted + capacity shrink |
| GPM sample + device cache pruning | `nvml.rs` | Per-tick active handle tracking → frees stale `nvmlGpmSample_t` + removes `DeviceInfo`, no leaks across repeated MIG reconfigs |
| NVML sample buffer shrink-to-fit | `nvml.rs` | Auto-shrinks when capacity > floor×8, prevents shrink↔resize thrashing on oscillating process/sample counts |
| DeviceInfo cache (one-time) + `Rc<str>` | `nvml.rs` | Static info cached on first call, clone only bumps reference count (zero heap allocation) |
| Process name caching + dead PID cleanup | `nvml.rs` | `/proc/{pid}/comm` I/O cached (`HashMap<u32, Rc<str>>`), dead PIDs not in current top-5 auto-removed each tick, cache hit returns `Rc::clone()` only (zero heap alloc) |
| `throttle_reasons` `Cow<'static, str>` | `nvml.rs` | "None", "Idle" etc. frequent single flags use `Cow::Borrowed` zero-alloc, only compound flags use `Cow::Owned` |
| Top Processes PID liveness carry-forward | `app.rs` | On NVML process API intermittent failure, checks `/proc/{pid}` existence and retains only alive processes, terminated processes removed immediately |
| VRAM carry-forward TTL (3 attempts) | `app.rs` | Retains previous VRAM on `memory_info()` up to 3 consecutive failures, switches to None("N/A") beyond that → prevents indefinite stale high value display |
| `/proc/{pid}` path buffer reuse | `app.rs` | Reuses `proc_path_buf: String` instead of per-PID format! heap allocation, prevents ~2.7 billion allocations over 300 hours |
| datetime format cache | `dashboard.rs` | `thread_local!` cache re-formats only when second changes (saves 1 String allocation/tick) |
| device_cache defensive shrink | `nvml.rs` | Prevents unbounded HashMap capacity growth on repeated MIG reconfigs → auto-shrink when `capacity > len*4` |
| `gpm_prev_samples` defensive shrink | `nvml.rs` | Same shrink heuristic applied to GPM sample HashMap → auto-shrink when `capacity > len*4`, reclaims memory on repeated MIG reconfigs |
| proc_name_cache HashSet-based pruning | `nvml.rs` | Per-tick dead PID pruning changed from O(n·m) nested iteration → `HashSet<u32>` O(n+m) lookup, consistent performance as process count grows |
| sysinfo targeted refresh | `main.rs` | Only `refresh_cpu_usage()` + `refresh_memory()` called, no process accumulation |
| `active_handles` HashSet reuse | `nvml.rs` | `RefCell<HashSet<usize>>` reuse + O(1) contains lookup, prune_stale_caches O(n²)→O(n) |
| history/vram_fail_count UUID HashSet cleanup | `app.rs` | GPU removal detection changes double `.any()` O(n×m) → HashSet O(n) single pass |
| Sparkline carry-forward TTL | `metrics.rs` | Indefinite last-value repeat on None caused stale sparklines → `none_counts[9]` per-metric array with 3-tick limit, stops pushing after expiry |
| `get_process_utilization` failure/idle distinction | `nvml.rs` | API failure `(0, 0)` indistinguishable from idle 0% → `Option<(u32, u32)>` return, idle 0% reported normally while failure proceeds to next fallback |
| `collect_all()` per-device error isolation | `nvml.rs` | Single GPU `device_by_index` error stopped entire metric collection → skips failed GPU only, remaining GPUs collected normally |
| GPM MIG Phase 2 removal | `nvml.rs` | `nvmlGpmMigSampleGet()` cross-tick corruption caused permanent `memory_info()` failure → eliminated, VRAM stability ensured |
| MIG display name caching | `nvml.rs` | MIG name `Rc<str>` created only on first tick, `Rc::clone()` refcount bump thereafter (eliminates per-tick heap alloc per MIG instance) |
| PID dedup HashSet reuse | `nvml.rs` | `RefCell<HashSet<u32>>` field reuse, zero allocation after first tick (clear only) |
| BAR_TABLE lookup table | `dashboard.rs` | `thread_local!` bar string table, rebuilds only on terminal resize, zero bar allocations per draw even on 128 cores |
| RAM chart fractional cell bg color | `dashboard.rs` | Used fractional character empty upper → `cell.set_bg(Color::Blue)` applied, ensures visual continuity at used-cached boundary |

### Long-Running Memory Profile

```
At startup:     ~4 MB RSS
After 5 min:    ~5-8 MB RSS (history buffers fill to 300 entries)
After 5 min:    No change (ring buffer → steady state maintained)
```

### Runtime Safety

| Protection Mechanism | Location | Description |
|---------------------|----------|-------------|
| Panic recovery hook | `main.rs` | On panic: auto-calls `disable_raw_mode()` + `LeaveAlternateScreen` → terminal state restored |
| Drift-corrected timer | `main.rs` | `Instant`-based elapsed measurement → subtracts work time from interval, prevents cumulative drift |
| Option-based graceful failure | `nvml.rs` | All extended metrics wrapped with `.ok()` → `None` ("N/A") on MIG/vGPU failure, no panics |
| `saturating_sub` time calc | `main.rs` | Even if work_time > interval, no negative values — immediately proceeds to next tick |

## Why Rust

- **Direct NVML FFI calls** — Raw C API access to bypass MIG limitations
- **Zero overhead** — Minimizes CPU/memory usage of the monitoring tool itself, no impact on GPU workloads
- **Single binary** — Deploy to cloud/container environments with just `scp` or `COPY`

## License

MIT
