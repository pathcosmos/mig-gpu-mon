mod app;
mod event;
mod gpu;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::KeyCode,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use sysinfo::System;

use app::App;
use event::{AppEvent, EventHandler};
use gpu::metrics::SystemMetrics;
use gpu::nvml::NvmlCollector;

#[derive(Parser)]
#[command(name = "mig-gpu-mon", about = "Real-time GPU monitor for MIG environments")]
struct Cli {
    /// Polling interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval: u64,

    /// Custom path to libnvidia-ml.so (override automatic detection)
    #[arg(long, value_name = "PATH")]
    nvml_path: Option<String>,
}

fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize NVML
    let collector = match NvmlCollector::new(cli.nvml_path.as_deref()) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Error: Failed to initialize NVML.\n");
            eprintln!("Possible causes:");
            eprintln!("  1. NVIDIA driver is not installed");
            eprintln!("     → Install with: sudo apt install nvidia-driver-XXX");
            eprintln!("       or check https://www.nvidia.com/drivers");
            eprintln!("  2. libnvidia-ml.so is not in the library path");
            eprintln!("     → Try: sudo ldconfig /usr/lib/x86_64-linux-gnu/");
            eprintln!("     → Or specify manually: mig-gpu-mon --nvml-path /path/to/libnvidia-ml.so.1");
            eprintln!("  3. Running inside a container without GPU access");
            eprintln!("     → Use: docker run --gpus all ...");
            eprintln!("     → Or:  docker run --runtime=nvidia -e NVIDIA_DRIVER_CAPABILITIES=compute,utility ...");
            eprintln!("  4. Cloud GPU instance (AWS, GCP, vast.io, RunPod)");
            eprintln!("     → Ensure nvidia-container-toolkit is installed on the host");
            eprintln!("     → Check: nvidia-smi  (should show GPU info)");
            eprintln!("     → If nvidia-smi works but this tool fails, try:");
            eprintln!("       mig-gpu-mon --nvml-path $(ldconfig -p | grep libnvidia-ml | awk '{{print $NF}}' | head -1)");
            if is_wsl() {
                eprintln!();
                eprintln!("  ** WSL 환경이 감지되었습니다 **");
                eprintln!("  5. WSL2를 사용 중인지 확인: wsl -l -v (VERSION이 2여야 함)");
                eprintln!("  6. Windows용 NVIDIA 드라이버를 최신 버전으로 설치/업데이트하세요");
                eprintln!("     → https://www.nvidia.com/drivers (Linux용이 아닌 Windows용)");
                eprintln!("  7. WSL 내에서 nvidia-smi가 동작하는지 확인하세요");
                eprintln!("  8. WSL1은 GPU 패스스루를 지원하지 않습니다");
                eprintln!("     → WSL1 → WSL2 변환: wsl --set-version <distro> 2");
            }
            std::process::exit(1);
        }
    };
    let driver = collector.driver_version();
    let cuda = collector.cuda_version();

    // Initialize sysinfo — only CPU + memory, not processes/disks/network
    let mut sys = System::new();
    // Prime CPU measurements (first call always returns 0)
    sys.refresh_cpu_usage();

    // Pre-allocate reusable buffer for CPU usage
    let cpu_count = sys.cpus().len();
    let mut cpu_buf: Vec<f32> = Vec::with_capacity(cpu_count);

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut app = App::new(driver, cuda);
    let events = EventHandler::new(cli.interval);

    // Main loop
    while app.running {
        // Collect GPU metrics
        match collector.collect_all() {
            Ok(metrics) => app.update_metrics(metrics),
            Err(e) => eprintln!("NVML error: {e}"),
        }

        // Collect system metrics — targeted refresh only
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        cpu_buf.clear();
        cpu_buf.extend(sys.cpus().iter().map(|c| c.cpu_usage()));
        let cpu_total = if cpu_buf.is_empty() {
            0.0
        } else {
            cpu_buf.iter().sum::<f32>() / cpu_buf.len() as f32
        };

        app.update_system_metrics(SystemMetrics {
            cpu_usage: cpu_buf.clone(),
            cpu_total,
            ram_used: sys.used_memory(),
            ram_total: sys.total_memory(),
            swap_used: sys.used_swap(),
            swap_total: sys.total_swap(),
        });

        // Draw
        terminal.draw(|f| ui::dashboard::draw(f, &app))?;

        // Handle events (blocks up to tick_rate)
        match events.next()? {
            AppEvent::Key(key) => match key.code {
                KeyCode::Tab | KeyCode::Down | KeyCode::Right => app.next_gpu(),
                KeyCode::BackTab | KeyCode::Up | KeyCode::Left => app.prev_gpu(),
                _ => {}
            },
            AppEvent::Quit => app.quit(),
            AppEvent::Tick => {}
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
