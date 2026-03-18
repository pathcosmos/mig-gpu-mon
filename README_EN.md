# mig-gpu-mon

[한국어](README.md) | **English**

A terminal TUI program for real-time monitoring of GPU metrics that `nvidia-smi` cannot provide in NVIDIA MIG (Multi-Instance GPU) environments.

Displays real-time sparkline graphs in btop/nvtop style, along with per-core CPU usage and system RAM monitoring.

> **Ubuntu-focused:** Development and testing are done on Ubuntu. Library search paths, error messages, and documentation are all written with Ubuntu as the primary target. It also works on RHEL-based distros, containers, and WSL2, but runs most smoothly on Ubuntu.

## Screen Layout

### ASCII Diagram

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
│ ▂▃▃▅▅▆▅▃▂▂▃▅▆▆▅▃              │                                     │ ← 25% (when PCIe available)
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
│   │   │   ├── CPU Cores         Min(4)    " CPU ({N} cores) {pct}% "
│   │   │   │   └── dynamic N-column bars   "{idx} ▮▮▯▯ {pct}%" (sorted by usage desc)
│   │   │   └── RAM / Swap        Length(4)  " Memory "
│   │   │       ├── RAM line                 "RAM ▮▮▯▯ {used}/{total} GiB ({pct}%)"
│   │   │       └── SWP line                 "SWP ▮▮▯▯ {used}/{total} GiB ({pct}%)"
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
│           ├── CPU Total {pct}%       sparkline   50%
│           └── RAM {u}/{t} GiB ({p}%) sparkline   50%
└── Footer                          Length(3)
```

### Color Coding

| Element | Color | Condition |
|---------|-------|-----------|
| CPU core bars | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| RAM bar | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| Swap bar | DarkGray / Yellow / Red | 0-20% / 20-50% / 50%+ |
| GPU Util sparkline | Green | — |
| Mem Ctrl sparkline | Blue | — |
| VRAM sparkline | Magenta | — |
| PCIe sparkline | LightCyan | Shown only when PCIe data available |
| CPU sparkline | Cyan | — |
| RAM sparkline | Yellow | — |
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
- **Top Processes** — displays top 5 processes by VRAM usage (PID, process name, MB)
- Parent GPU metrics (temperature, power, process count) displayed simultaneously
- **Clock Speeds** — Graphics/SM/Memory clocks (MHz) + Performance State (P0~P15)
- **PCIe Throughput** — Gen/Width + TX/RX transfer rates (MB/s), conditional sparkline graph
- **Encoder/Decoder Utilization** — NVENC/NVDEC usage (%)
- **ECC Status** — enabled state + Corrected/Uncorrected error counts
- **Temperature Thresholds** — Slowdown/Shutdown threshold display
- **Throttle Reasons** — Real-time GPU throttle cause display (PwrCap, HW-Therm, etc.)
- **Architecture & Compute Capability** — GPU architecture (Ampere, Hopper, etc.) + CUDA CC
- Per-core CPU usage (sorted by usage descending, dynamic multi-column bar graph adapting to terminal width)
- System RAM / Swap usage
- Time-series sparkline graphs for GPU Util / Mem Ctrl / **VRAM** / **PCIe** / CPU Total / RAM (current values in title)
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
| **Disk I/O** | **0** | No file reads/writes |
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
│   ├── running_compute_procs  ~0.5ms
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
│   └── SystemHistory                   ~5 KB    (2 VecDeque × 300 × 4-8B)
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
| CPU cores Vec reuse | `dashboard.rs` | Vec alloc per draw → `thread_local!` buffer reuse |
| `make_bar()` string | `dashboard.rs` | `.repeat()` 2× concatenation → `String::with_capacity` + push loop |
| HashMap uuid clone | `app.rs` | `uuid.clone()` every tick → `contains_key` then clone only on miss |
| GPU history auto-cleanup | `app.rs` | Unbounded HashMap growth on MIG reconfig/GPU removal → `retain()` removes orphan UUID entries |
| NVML sample buffer shrink | `nvml.rs` | grow-only buffer could grow unbounded → auto `shrink_to(needed×2)` when capacity > needed×4 |
| `format_pstate` zero-alloc | `nvml.rs` | `"P0".to_string()` per tick → returns `&'static str` (zero allocation) |
| `format_architecture` zero-alloc | `nvml.rs` | Same pattern: `"Ampere".to_string()` → `&'static str` |
| `format_throttle_reasons` Vec removal | `nvml.rs` | `Vec::new()` + `push` + `join()` → macro appends directly to `String` (eliminates Vec allocation) |

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
| GPU history auto-cleanup | `app.rs` | Orphan entries auto-deleted on MIG reconfig/GPU removal |
| NVML sample buffer shrink-to-fit | `nvml.rs` | Auto-shrinks when capacity > needed×4, recovers after transient spikes |
| DeviceInfo cache (one-time) | `nvml.rs` | Static info (arch, CC, etc.) cached on first call, zero allocation thereafter |
| sysinfo targeted refresh | `main.rs` | Only `refresh_cpu_usage()` + `refresh_memory()` called, no process accumulation |

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
