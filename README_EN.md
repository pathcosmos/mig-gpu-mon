# mig-gpu-mon

[한국어](README.md) | **English**

A terminal TUI program for real-time monitoring of GPU metrics that `nvidia-smi` cannot provide in NVIDIA MIG (Multi-Instance GPU) environments.

Displays real-time graphs and gauges in btop/nvtop style, along with per-core CPU usage and system RAM monitoring.

> **Ubuntu-focused:** Development and testing are done on Ubuntu. Library search paths, error messages, and documentation are all written with Ubuntu as the primary target. It also works on RHEL-based distros, containers, and WSL2, but runs most smoothly on Ubuntu.

## Screen Layout

### ASCII Diagram

```
┌─ mig-gpu-mon ──────────────────────────────────────────────────────────┐
│ MIG GPU Monitor | Driver: 535.129.03 | CUDA: 12.2 | GPUs: 3           │ ← Header
├─ CPU (64 cores) 23.4% ─────────┬─ Devices ────────────────────────────┤
│  0 ▮▮▯▯▯▯▯  12%  32 ▮▯▯▯▯  3% │ > MIG 0 (GPU 0: A100) GPU:45% MEM:… │ ↑ 25%
│  1 ▮▮▮▯▯▯▯  34%  33 ▮▮▯▯▯ 18% │   MIG 1 (GPU 0: A100) GPU:12% MEM:… │ ↓
│  2 ▮▮▮▮▯▯▯  52%  34 ▮▯▯▯▯  5% ├─ Detail ─────────────────────────────┤    ← Top 45%
│  ...                            │ Name: MIG 0 (GPU 0: A100-SXM4-80GB) │ ↑
├─ Memory ────────────────────────┤ UUID: MIG-a1b2c3d4e5f6...           │ │
│ RAM ▮▮▮▮▮▯▯ 89.2/256.0 GiB … │ VRAM 12288 MB / 20480 MB (60.0%)    │ │ 40%
│ SWP ▮▯▯▯▯▯▯  2.1/32.0 GiB  … │ GPU Util: 45%  Mem Util: 38%        │ │
│                                 │ Temp: 62°C  Power: 127.3W / 300.0W  │ │
│                                 │ Processes: 2                         │ ↓
│                                 ├─ VRAM Top 5 Processes ───────────────┤
│                                 │ PID     Process         VRAM        │ ↑
│                                 │ 12345   python3          8192 MB    │ │ 35%
│                                 │ 12400   pt_main_thread   4096 MB    │ │
│                                 │   No more processes                  │ ↓
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
│   │   │   └── RAM / Swap        Length(5)  " Memory "
│   │   │       ├── RAM line                 "RAM ▮▮▯▯ {used}/{total} GiB ({pct}%)"
│   │   │       └── SWP line                 "SWP ▮▮▯▯ {used}/{total} GiB ({pct}%)"
│   │   └── GPU Panel     50%
│   │       ├── Device List        25%       " Devices "
│   │       │   └── "{>} {MIG|GPU} {idx}: {name} | GPU:{pct}% MEM:{pct}%"
│   │       ├── GPU Detail         40%       " Detail "
│   │       │   ├── Name:     {name}
│   │       │   ├── UUID:     {uuid (max 20 chars)}
│   │       │   ├── VRAM     {used} MB / {total} MB ({pct}%)
│   │       │   ├── GPU Util: {pct}%
│   │       │   ├── Mem Util: {pct}%
│   │       │   ├── SM Util:  {pct}%          (MIG only)
│   │       │   ├── Temp:     {val}°C         (if available)
│   │       │   ├── Power:    {usage}W / {limit}W  (if available)
│   │       │   └── Processes: {count}
│   │       └── VRAM Top 5 Procs   35%       " VRAM Top 5 Processes "
│   │           ├── Header: PID / Process / VRAM
│   │           └── {pid} {name (max 15)} {vram} MB  (top 5 by VRAM desc)
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

### Color Coding

| Element | Color | Condition |
|---------|-------|-----------|
| CPU core bars | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| RAM bar | Green / Yellow / Red | 0-50% / 50-80% / 80%+ |
| Swap bar | DarkGray / Yellow / Red | 0-20% / 20-50% / 50%+ |
| GPU Util sparkline | Green | — |
| Mem Util sparkline | Blue | — |
| CPU sparkline | Cyan | — |
| RAM sparkline | Yellow | — |
| GPU % gauge | Green | — |
| VRAM gauge | Magenta | — |
| VRAM % (Detail) | Green / Yellow / Red | 0-70% / 70-90% / 90%+ |
| RAM gauge | Yellow | — |
| Temp | Green / Yellow / Red | 0-60°C / 60-80°C / 80°C+ |
| Selected GPU | Green + Bold | — |
| Header | Cyan + Bold | — |

## Why

In MIG environments, `nvidia-smi` cannot display key metrics such as GPU Utilization and Memory Utilization.
This is because `nvmlDeviceGetUtilizationRates()` returns `NVML_ERROR_NOT_SUPPORTED` for MIG device handles.

This tool bypasses that limitation by calling the NVML C API directly:

1. `nvmlDeviceGetMigDeviceHandleByIndex()` — Obtain MIG instance handle
2. `nvmlDeviceGetProcessUtilization()` — Collect per-process SM/Memory utilization
3. Aggregate per-process values to compute instance-level GPU Util / Memory Util / SM Util

## Features

- Real-time per-MIG-instance GPU Util, Memory Util, SM Util, and VRAM usage
- **VRAM Top 5 Processes** — displays top 5 processes by VRAM usage (PID, process name, MB)
- Parent GPU metrics (temperature, power, process count) displayed simultaneously
- Per-core CPU usage (sorted by usage descending, dynamic multi-column bar graph adapting to terminal width)
- System RAM / Swap usage
- Time-series sparkline graphs for GPU Util / Memory Util / CPU Total / RAM
- GPU Util / VRAM / RAM gauges
- Switch between GPU/MIG instances with Tab/arrow keys
- Single binary deployment (~1.5MB, dynamically links libc — no separate runtime install needed)

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
    1. Collect GPU metrics (NVML API)
       - Physical GPU: utilization_rates(), memory_info(), temperature(), ...
       - MIG instance: on utilization_rates() failure
         → fallback to nvmlDeviceGetProcessUtilization() for SM/Mem util aggregation
    2. Collect system metrics (sysinfo)
       - Per-core CPU usage, total RAM/Swap
    3. TUI rendering (ratatui)
    4. Wait for events (crossterm poll, blocks for interval duration)
       - Process key input or tick → next loop iteration
}
```

## MIG Utilization Collection Mechanism

How GPU/Memory utilization is obtained in MIG environments:

```
nvmlDeviceGetMigDeviceHandleByIndex(parent, idx)
    → mig_handle

nvmlDeviceGetUtilizationRates(mig_handle)
    → Success: use gpu_util, memory_util
    → Failure (NVML_ERROR_NOT_SUPPORTED):
        nvmlDeviceGetProcessUtilization(mig_handle, samples, &count, 0)
            → 1st call: count=0 → NVML_ERROR_INSUFFICIENT_SIZE, count returns required size
            → 2nd call: pass buffer → collect per-process smUtil, memUtil
            → Aggregate max(smUtil), max(memUtil) for instance-level values
```

## Performance Optimization

Resource usage is minimized so that the monitoring tool itself does not affect GPU workloads.

### Expected Resource Consumption

Estimates for default settings (1-second interval), 1 GPU + 2 MIG instances:

| Resource | Expected Usage | Notes |
|----------|---------------|-------|
| **CPU** | **0.5~2% (per core)** | ~5-18ms active time per tick, ~982-995ms sleep |
| **RSS Memory** | **4~8 MB** | Binary + libnvidia-ml.so + history buffers + TUI buffers |
| **GPU Compute** | **0% (unused)** | NVML is read-only driver IPC, no CUDA context created |
| **GPU VRAM** | **0 MB (unused)** | No GPU memory allocation |
| **Disk I/O** | **0** | No file reads/writes |
| **Network** | **0** | No network communication |

#### Per-Tick Time Breakdown (1-second interval)

```
1 tick = 1000ms
├── NVML API calls        ~5-15ms   Driver IPC (5-7 queries per GPU)
│   ├── device_by_index        ~0.1ms
│   ├── utilization_rates      ~0.5ms
│   ├── memory_info            ~0.5ms
│   ├── temperature            ~0.3ms
│   ├── power_usage            ~0.3ms
│   ├── power_management_limit ~0.3ms
│   ├── running_compute_procs  ~0.5ms
│   └── (MIG) process_util     ~1-3ms   Per MIG instance, fallback only
├── sysinfo refresh       ~0.1-0.3ms
│   ├── refresh_cpu_usage      ~0.1ms   Reads /proc/stat
│   └── refresh_memory         ~0.05ms  Reads /proc/meminfo
├── TUI rendering         ~0.5-2ms   ratatui diff buffer + ANSI output
├── Event wait (sleep)    ~982-995ms  crossterm poll, kernel scheduling
└── Total active time     ~5-18ms    = CPU 0.5-1.8%
```

#### RSS Memory Breakdown

```
Total RSS ~4-8 MB
├── Binary code/data segments          ~1.4 MB   (mmap)
├── libnvidia-ml.so shared library     ~2-4 MB   (mmap, shared with system)
├── History ring buffers               ~80 KB
│   ├── MetricsHistory per GPU          ~14 KB   (6 VecDeque × 300 × 4-8B)
│   │   (× 3 devices = ~42 KB)
│   └── SystemHistory                   ~5 KB    (2 VecDeque × 300 × 4-8B)
├── ratatui Terminal double buffer     ~50-400 KB (proportional to terminal size)
│   (80×24: ~77KB, 200×50: ~400KB)
├── sysinfo System struct              ~30-50 KB  (CPU only, no processes)
├── Reusable buffers                    ~5 KB
│   ├── thread_local sparkline buf      ~2.4 KB
│   ├── proc_sample_buf                 ~1 KB
│   └── cpu_buf                         ~0.3 KB
└── HashMap, String caches, etc.        ~5-10 KB
```

#### Resource Comparison by Interval

| Interval | CPU Usage | Characteristics |
|----------|-----------|-----------------|
| `500ms` | ~1-3.5% | Fast response, slightly increased monitoring overhead |
| `1000ms` (default) | ~0.5-1.8% | Balanced default |
| `2000ms` | ~0.3-0.9% | Resource saving, for large-scale clusters |
| `5000ms` | ~0.1-0.4% | Minimum overhead, for long-term observation |

> RSS memory is the same regardless of interval. Since the history entry count (300) is fixed,
> longer intervals record a longer time range (1s: 5min, 2s: 10min, 5s: 25min).

### Optimization Details: Memory

| Optimization | Location | Before → After |
|-------------|----------|----------------|
| `VecDeque` ring buffer | `metrics.rs` | `Vec::remove(0)` O(n) memmove → `VecDeque::pop_front()` O(1) |
| Device info cache | `nvml.rs` | NVML API + String alloc every tick → `RefCell<HashMap>` first call only, cache hit thereafter |
| Process sample buffer | `nvml.rs` | `vec![zeroed(); N]` alloc/dealloc per MIG call → `RefCell<Vec>` grow-only reuse |
| CPU buffer reuse | `main.rs` | `Vec::new()` every tick → `cpu_buf.clear()` + extend (capacity retained) |
| Sparkline conversion buffer | `dashboard.rs` | 4× `Vec<u64>` alloc per draw → `thread_local!` single scratch reuse |
| `make_bar()` string | `dashboard.rs` | `.repeat()` 2× concatenation → `String::with_capacity` + push loop |
| HashMap uuid clone | `app.rs` | `uuid.clone()` every tick → `contains_key` then clone only on miss |

### Optimization Details: CPU (Minimize System Calls)

| Optimization | Location | Effect |
|-------------|----------|--------|
| `System::new()` | `main.rs` | Eliminates full process/disk/network scan vs `new_all()` |
| Targeted refresh | `main.rs` | Only `refresh_cpu_usage()` + `refresh_memory()` — reads just /proc/stat and /proc/meminfo |
| Default interval 1000ms | `main.rs` | Halves all syscall + NVML call frequency vs 500ms |
| CPU priming | `main.rs` | Prevents sysinfo's first `refresh_cpu_usage()` returning 0% — one pre-call at init |

### Optimization Details: GPU (Minimize NVML Calls)

| Optimization | Location | Effect |
|-------------|----------|--------|
| `utilization_rates()` first | `nvml.rs` | Try even on MIG, fallback to process util only on failure (saves 2 extra IPCs) |
| Process util 2-pass | `nvml.rs` | 1st call with count=0 to get size, 2nd call to fetch data — prevents over-allocation |
| `RefCell` interior mutability | `nvml.rs` | Allows cache/buffer mutation with `&self` while NVML handles borrow, no borrow checker conflicts |
| Zero GPU resource usage | Design | NVML is read-only driver query — no CUDA context, no VRAM allocation |

### Optimization Details: Binary Size

| Setting | Value | Effect |
|---------|-------|--------|
| `opt-level` | 3 | Maximum optimization |
| `lto` | true | Link-Time Optimization, dead code elimination |
| `strip` | true | Complete debug symbol removal |
| `tokio` removal | — | No async needed, synchronous event loop suffices — saves ~200KB |
| Final size | **~1.5MB** | Single binary (dynamically links libc) |

## Why Rust

- **Direct NVML FFI calls** — Raw C API access to bypass MIG limitations
- **Zero overhead** — Minimizes CPU/memory usage of the monitoring tool itself, no impact on GPU workloads
- **Single binary** — Deploy to cloud/container environments with just `scp` or `COPY`

## License

MIT
