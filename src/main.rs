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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize NVML
    let collector = match NvmlCollector::new() {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Error: Failed to initialize NVML.\n");
            eprintln!("Possible causes:");
            eprintln!("  1. NVIDIA driver is not installed");
            eprintln!("     → Install with: sudo apt install nvidia-driver-XXX");
            eprintln!("       or check https://www.nvidia.com/drivers");
            eprintln!("  2. libnvidia-ml.so is not in the library path");
            eprintln!("     → Try: sudo ldconfig /usr/lib/x86_64-linux-gnu/");
            eprintln!("  3. Running inside a container without GPU access");
            eprintln!("     → Use: docker run --gpus all ...");
            eprintln!("           or nvidia-docker");
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
