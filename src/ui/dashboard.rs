use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Sparkline},
    Frame,
};

use crate::app::App;

// Reusable scratch buffer for sparkline u64 conversion.
// Avoids allocation per draw call. Thread-local since draw is single-threaded.
thread_local! {
    static SPARK_BUF: std::cell::RefCell<Vec<u64>> = std::cell::RefCell::new(Vec::with_capacity(300));
    static CORE_SORT_BUF: std::cell::RefCell<Vec<(usize, f32)>> = std::cell::RefCell::new(Vec::with_capacity(128));
}

/// Convert VecDeque<u32> to &[u64] via thread-local scratch buffer, then call f.
fn with_spark_data_u32(src: &std::collections::VecDeque<u32>, f: impl FnOnce(&[u64])) {
    SPARK_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(src.iter().map(|&v| v as u64));
        f(&buf);
    });
}

fn with_spark_data_f32(src: &std::collections::VecDeque<f32>, f: impl FnOnce(&[u64])) {
    SPARK_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(src.iter().map(|&v| v as u64));
        f(&buf);
    });
}

fn with_spark_data_f64(src: &std::collections::VecDeque<f64>, f: impl FnOnce(&[u64])) {
    SPARK_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(src.iter().map(|&v| v as u64));
        f(&buf);
    });
}

fn with_spark_data_u64(src: &std::collections::VecDeque<u64>, f: impl FnOnce(&[u64])) {
    SPARK_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(src.iter().copied());
        f(&buf);
    });
}

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let header_text = format!(
        " MIG GPU Monitor | Driver: {} | CUDA: {} | GPUs: {}",
        app.driver_version,
        app.cuda_version,
        app.metrics.len()
    );
    let now = chrono::Local::now()
        .format("%Y-%m-%d %I:%M:%S %p")
        .to_string();
    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" mig-gpu-mon ")
                .title(Line::from(format!(" {} ", now)).alignment(Alignment::Right)),
        );
    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    draw_system_panel(f, app, top_cols[0]);
    draw_gpu_panel(f, app, top_cols[1]);
    draw_charts(f, app, rows[1]);
}

fn draw_system_panel(f: &mut Frame, app: &App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(4)])
        .split(area);

    draw_cpu_cores(f, app, sections[0]);
    draw_ram_swap(f, app, sections[1]);
}

fn draw_cpu_cores(f: &mut Frame, app: &App, area: Rect) {
    let sys = match &app.system_metrics {
        Some(s) => s,
        None => {
            let block = Block::default().borders(Borders::ALL).title(" CPU ");
            f.render_widget(block, area);
            return;
        }
    };

    let total_label = format!(
        " CPU ({} cores) {:.1}% ",
        sys.cpu_usage.len(),
        sys.cpu_total
    );
    let block = Block::default().borders(Borders::ALL).title(total_label);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 || sys.cpu_usage.is_empty() {
        return;
    }

    // Sort cores by usage descending (keep original core index) — reuse thread-local buffer
    CORE_SORT_BUF.with(|buf| {
        let mut sorted_cores = buf.borrow_mut();
        sorted_cores.clear();
        sorted_cores.extend(sys.cpu_usage.iter().enumerate().map(|(i, &u)| (i, u)));
        sorted_cores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Dynamic column count: each column = 3(idx) + 1(space) + bar + 5(pct) = 9 + bar
        // Non-first columns add 1-char separator, so usable = width - (cols-1)
        let min_col_width: u16 = 12;
        let num_cols = (inner.width / min_col_width).max(1) as usize;
        let usable_width = inner.width.saturating_sub((num_cols - 1) as u16);
        let col_width = usable_width / num_cols as u16;
        let bar_width = col_width.saturating_sub(8) as usize;

        let max_rows = inner.height as usize;
        let max_visible = max_rows * num_cols;
        let visible_count = sorted_cores.len().min(max_visible);

        let rows_needed = visible_count.div_ceil(num_cols);
        let mut lines: Vec<Line> = Vec::with_capacity(rows_needed);

        for row in 0..rows_needed {
            let mut spans = Vec::with_capacity(num_cols * 4);

            for col in 0..num_cols {
                let idx = col * rows_needed + row;
                if idx >= visible_count {
                    break;
                }
                let (core_idx, usage) = sorted_cores[idx];
                let color = cpu_color(usage);

                if col > 0 {
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled(
                    format!("{:>3}", core_idx),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    make_bar(usage, bar_width),
                    Style::default().fg(color),
                ));
                spans.push(Span::styled(
                    format!("{:>4.0}%", usage),
                    Style::default().fg(color),
                ));
            }

            lines.push(Line::from(spans));
        }

        let para = Paragraph::new(lines);
        f.render_widget(para, inner);
    });
}

fn cpu_color(usage: f32) -> Color {
    if usage > 80.0 {
        Color::Red
    } else if usage > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn make_bar(percent: f32, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let filled = ((percent / 100.0) * width as f32).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    // Pre-sized string: " " + filled * "▮"(3 bytes) + empty * "▯"(3 bytes)
    let mut s = String::with_capacity(1 + (filled + empty) * 3);
    s.push(' ');
    for _ in 0..filled {
        s.push('▮');
    }
    for _ in 0..empty {
        s.push('▯');
    }
    s
}

fn draw_ram_swap(f: &mut Frame, app: &App, area: Rect) {
    let sys = match &app.system_metrics {
        Some(s) => s,
        None => {
            let block = Block::default().borders(Borders::ALL).title(" Memory ");
            f.render_widget(block, area);
            return;
        }
    };

    let block = Block::default().borders(Borders::ALL).title(" Memory ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let ram_pct = sys.ram_percent();
    let ram_color = if ram_pct > 80.0 {
        Color::Red
    } else if ram_pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let ram_bar_width = rows[0].width.saturating_sub(30) as usize;
    let ram_line = Line::from(vec![
        Span::styled(
            "RAM",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            make_bar(ram_pct as f32, ram_bar_width),
            Style::default().fg(ram_color),
        ),
        Span::styled(
            format!(
                " {:.1}/{:.1} GiB ({:.1}%)",
                sys.ram_used_gb(),
                sys.ram_total_gb(),
                ram_pct
            ),
            Style::default().fg(Color::White),
        ),
    ]);
    f.render_widget(Paragraph::new(ram_line), rows[0]);

    if sys.swap_total > 0 {
        let swap_pct = sys.swap_percent();
        let swap_color = if swap_pct > 50.0 {
            Color::Red
        } else if swap_pct > 20.0 {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        let swap_line = Line::from(vec![
            Span::styled(
                "SWP",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                make_bar(swap_pct as f32, ram_bar_width),
                Style::default().fg(swap_color),
            ),
            Span::styled(
                format!(
                    " {:.1}/{:.1} GiB ({:.1}%)",
                    sys.swap_used_gb(),
                    sys.swap_total_gb(),
                    swap_pct
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        f.render_widget(Paragraph::new(swap_line), rows[1]);
    }
}

fn draw_gpu_panel(f: &mut Frame, app: &App, area: Rect) {
    if app.metrics.is_empty() {
        let msg = Paragraph::new("No GPU devices detected. Waiting...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(msg, area);
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(50),
            Constraint::Percentage(30),
        ])
        .split(area);

    draw_gpu_list(f, app, sections[0]);
    draw_gpu_detail(f, app, sections[1]);
    draw_vram_top_processes(f, app, sections[2]);
}

fn draw_gpu_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .metrics
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let prefix = if m.is_mig_instance { "  MIG" } else { "GPU" };
            let indicator = if i == app.selected_gpu { ">" } else { " " };
            let style = if i == app.selected_gpu {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let gpu_str = m.gpu_util.map_or("N/A".to_string(), |v| format!("{}%", v));
            let mem_str = m
                .memory_util
                .map_or("N/A".to_string(), |v| format!("{}%", v));
            ListItem::new(format!(
                "{} {} {}: {} | GPU:{} Mem:{}",
                indicator, prefix, m.index, m.name, gpu_str, mem_str
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Devices "));
    f.render_widget(list, area);
}

fn draw_gpu_detail(f: &mut Frame, app: &App, area: Rect) {
    let m = match app.selected_metrics() {
        Some(m) => m,
        None => return,
    };

    let mut lines = Vec::with_capacity(14);

    // Line 1: Name (with parent GPU index for MIG)
    let name_display = if m.is_mig_instance {
        if let Some(parent) = m.parent_gpu_index {
            format!("{} [Parent: GPU {}]", m.name, parent)
        } else {
            m.name.clone()
        }
    } else {
        m.name.clone()
    };
    lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
        Span::styled(name_display, Style::default().fg(Color::White)),
    ]));

    // Line 2: UUID + static info (Arch, CC)
    let mut uuid_spans = vec![
        Span::styled("UUID: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &m.uuid[..m.uuid.len().min(20)],
            Style::default().fg(Color::White),
        ),
    ];
    if let Some(arch) = m.architecture {
        uuid_spans.push(Span::styled(
            "  Arch: ",
            Style::default().fg(Color::DarkGray),
        ));
        uuid_spans.push(Span::styled(arch, Style::default().fg(Color::Cyan)));
    }
    if let Some(ref cc) = m.compute_capability {
        uuid_spans.push(Span::styled("  CC: ", Style::default().fg(Color::DarkGray)));
        uuid_spans.push(Span::styled(cc.as_str(), Style::default().fg(Color::Cyan)));
    }
    lines.push(Line::from(uuid_spans));

    // Line 3: blank
    lines.push(Line::from(""));

    // Line 4: VRAM
    if let (Some(used_mb), Some(total_mb), Some(pct)) =
        (m.memory_used_mb(), m.memory_total_mb(), m.memory_percent())
    {
        lines.push(Line::from(vec![
            Span::styled(
                "VRAM ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} MB", used_mb),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" / {} MB ", total_mb),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("({:.1}%)", pct),
                Style::default().fg(vram_pct_color(pct)),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "VRAM ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("N/A", Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Line 5: GPU / Mem / SM util (compact horizontal)
    let gpu_util_str = m.gpu_util.map_or("N/A".to_string(), |v| format!("{}%", v));
    let mem_util_str = m
        .memory_util
        .map_or("N/A".to_string(), |v| format!("{}%", v));
    let mut util_spans = vec![
        Span::styled("GPU: ", Style::default().fg(Color::Green)),
        Span::styled(
            gpu_util_str,
            Style::default().fg(if m.gpu_util.is_some() {
                Color::White
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled("  Mem: ", Style::default().fg(Color::Blue)),
        Span::styled(
            mem_util_str,
            Style::default().fg(if m.memory_util.is_some() {
                Color::White
            } else {
                Color::DarkGray
            }),
        ),
    ];
    if let Some(sm) = m.sm_util {
        util_spans.push(Span::styled("  SM: ", Style::default().fg(Color::Magenta)));
        util_spans.push(Span::styled(
            format!("{}%", sm),
            Style::default().fg(Color::White),
        ));
    }
    lines.push(Line::from(util_spans));

    // Line 6: Encoder / Decoder
    if m.encoder_util.is_some() || m.decoder_util.is_some() {
        let mut enc_spans = Vec::new();
        if let Some(enc) = m.encoder_util {
            enc_spans.push(Span::styled("Enc: ", Style::default().fg(Color::Magenta)));
            enc_spans.push(Span::styled(
                format!("{}%", enc),
                Style::default().fg(Color::White),
            ));
        }
        if let Some(dec) = m.decoder_util {
            if !enc_spans.is_empty() {
                enc_spans.push(Span::raw("  "));
            }
            enc_spans.push(Span::styled("Dec: ", Style::default().fg(Color::Magenta)));
            enc_spans.push(Span::styled(
                format!("{}%", dec),
                Style::default().fg(Color::White),
            ));
        }
        lines.push(Line::from(enc_spans));
    }

    // Line 7: Clock speeds + PState
    if m.clock_graphics_mhz.is_some() || m.performance_state.is_some() {
        let mut clk_spans = Vec::new();
        if let (Some(gfx), Some(sm), Some(mem)) =
            (m.clock_graphics_mhz, m.clock_sm_mhz, m.clock_memory_mhz)
        {
            clk_spans.push(Span::styled("Clk: ", Style::default().fg(Color::DarkGray)));
            clk_spans.push(Span::styled(
                format!("{}/{}/{} MHz", gfx, sm, mem),
                Style::default().fg(Color::Cyan),
            ));
        } else if let Some(gfx) = m.clock_graphics_mhz {
            clk_spans.push(Span::styled("Clk: ", Style::default().fg(Color::DarkGray)));
            clk_spans.push(Span::styled(
                format!("{} MHz", gfx),
                Style::default().fg(Color::Cyan),
            ));
        }
        if let Some(ps) = m.performance_state {
            if !clk_spans.is_empty() {
                clk_spans.push(Span::raw("  "));
            }
            let ps_color = pstate_color(ps);
            clk_spans.push(Span::styled(ps, Style::default().fg(ps_color)));
        }
        if !clk_spans.is_empty() {
            lines.push(Line::from(clk_spans));
        }
    }

    // Line 8: Temp + thresholds + Power
    if m.temperature.is_some() || m.power_usage.is_some() {
        let mut tp_spans = Vec::new();
        if let Some(temp) = m.temperature {
            let temp_color = temp_color(temp);
            tp_spans.push(Span::styled("Temp: ", Style::default().fg(Color::DarkGray)));
            tp_spans.push(Span::styled(
                format!("{}°C", temp),
                Style::default().fg(temp_color),
            ));
            // Show thresholds inline
            if let Some(sd) = m.temp_slowdown {
                tp_spans.push(Span::styled(
                    format!(" (↓{}", sd),
                    Style::default().fg(Color::DarkGray),
                ));
                if let Some(sh) = m.temp_shutdown {
                    tp_spans.push(Span::styled(
                        format!(" ✕{})", sh),
                        Style::default().fg(Color::DarkGray),
                    ));
                } else {
                    tp_spans.push(Span::styled(")", Style::default().fg(Color::DarkGray)));
                }
            }
        }
        if let (Some(usage), Some(limit)) = (m.power_usage_w(), m.power_limit_w()) {
            if !tp_spans.is_empty() {
                tp_spans.push(Span::raw("  "));
            }
            tp_spans.push(Span::styled(
                "Power: ",
                Style::default().fg(Color::DarkGray),
            ));
            tp_spans.push(Span::styled(
                format!("{:.1}/{:.1}W", usage, limit),
                Style::default().fg(Color::Magenta),
            ));
        }
        if !tp_spans.is_empty() {
            lines.push(Line::from(tp_spans));
        }
    }

    // Line 9: PCIe
    if m.pcie_gen.is_some() || m.pcie_tx_kbps.is_some() {
        let mut pcie_spans = vec![Span::styled("PCIe: ", Style::default().fg(Color::DarkGray))];
        if let (Some(gen), Some(width)) = (m.pcie_gen, m.pcie_width) {
            pcie_spans.push(Span::styled(
                format!("Gen{} x{}", gen, width),
                Style::default().fg(Color::LightCyan),
            ));
        }
        if let (Some(tx), Some(rx)) = (m.pcie_tx_mbps(), m.pcie_rx_mbps()) {
            if pcie_spans.len() > 1 {
                pcie_spans.push(Span::raw("  "));
            }
            pcie_spans.push(Span::styled(
                format!("TX:{:.1} RX:{:.1} MB/s", tx, rx),
                Style::default().fg(Color::LightCyan),
            ));
        }
        if pcie_spans.len() > 1 {
            lines.push(Line::from(pcie_spans));
        }
    }

    // Line 10: ECC
    if let Some(ecc_on) = m.ecc_enabled {
        let mut ecc_spans = vec![
            Span::styled("ECC: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if ecc_on { "On" } else { "Off" },
                Style::default().fg(if ecc_on {
                    Color::Green
                } else {
                    Color::DarkGray
                }),
            ),
        ];
        if let Some(corr) = m.ecc_errors_corrected {
            ecc_spans.push(Span::styled(
                format!("  Corr:{}", corr),
                Style::default().fg(if corr == 0 {
                    Color::Green
                } else {
                    Color::Yellow
                }),
            ));
        }
        if let Some(uncorr) = m.ecc_errors_uncorrected {
            let uncorr_style = if uncorr > 0 {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            ecc_spans.push(Span::styled(format!("  Uncorr:{}", uncorr), uncorr_style));
        }
        lines.push(Line::from(ecc_spans));
    }

    // Line 11: Throttle
    if let Some(ref throttle) = m.throttle_reasons {
        let throttle_style = if throttle == "None" {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        };
        lines.push(Line::from(vec![
            Span::styled("Throttle: ", Style::default().fg(Color::DarkGray)),
            Span::styled(throttle.as_str(), throttle_style),
        ]));
    }

    // Line 12: Processes
    lines.push(Line::from(vec![
        Span::styled("Processes: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", m.process_count),
            Style::default().fg(Color::White),
        ),
    ]));

    let detail =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Detail "));
    f.render_widget(detail, area);
}

fn draw_charts(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_gpu_charts(f, app, cols[0]);
    draw_system_charts(f, app, cols[1]);
}

fn draw_gpu_charts(f: &mut Frame, app: &App, area: Rect) {
    let history = match app.selected_history() {
        Some(h) => h,
        None => {
            let block = Block::default().borders(Borders::ALL).title(" GPU Charts ");
            f.render_widget(block, area);
            return;
        }
    };

    // Conditionally add PCIe sparkline if data is available
    let has_pcie = !history.pcie_tx_kbps.is_empty();

    let rows = if has_pcie {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(area)
    };

    // GPU Utilization sparkline
    let gpu_title = app
        .selected_metrics()
        .map(|m| {
            m.gpu_util.map_or(" GPU Util N/A ".to_string(), |v| {
                format!(" GPU Util {}% ", v)
            })
        })
        .unwrap_or_else(|| " GPU Util ".to_string());
    with_spark_data_u32(&history.gpu_util, |data| {
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(gpu_title.as_str()),
            )
            .data(data)
            .max(100)
            .style(Style::default().fg(Color::Green));
        f.render_widget(sparkline, rows[0]);
    });

    // Memory Controller Utilization sparkline
    let mem_ctrl_title = app
        .selected_metrics()
        .map(|m| {
            m.memory_util.map_or(" Mem Ctrl N/A ".to_string(), |v| {
                format!(" Mem Ctrl {}% ", v)
            })
        })
        .unwrap_or_else(|| " Mem Ctrl ".to_string());
    with_spark_data_u32(&history.memory_util, |data| {
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(mem_ctrl_title.as_str()),
            )
            .data(data)
            .max(100)
            .style(Style::default().fg(Color::Blue));
        f.render_widget(sparkline, rows[1]);
    });

    // VRAM Usage sparkline
    let vram_title = app
        .selected_metrics()
        .map(
            |m| match (m.memory_used_mb(), m.memory_total_mb(), m.memory_percent()) {
                (Some(used), Some(total), Some(pct)) => {
                    format!(" VRAM {}/{} MB ({:.1}%) ", used, total, pct)
                }
                _ => " VRAM N/A ".to_string(),
            },
        )
        .unwrap_or_else(|| " VRAM ".to_string());
    let vram_max = app
        .selected_metrics()
        .and_then(|m| m.memory_total_mb())
        .unwrap_or(1);
    with_spark_data_u64(&history.memory_used_mb, |data| {
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(vram_title.as_str()),
            )
            .data(data)
            .max(vram_max)
            .style(Style::default().fg(Color::Magenta));
        f.render_widget(sparkline, rows[2]);
    });

    // PCIe throughput sparkline (conditional)
    if has_pcie {
        let pcie_title = app
            .selected_metrics()
            .and_then(|m| {
                let tx = m.pcie_tx_mbps()?;
                let rx = m.pcie_rx_mbps()?;
                Some(format!(" PCIe TX:{:.1} RX:{:.1} MB/s ", tx, rx))
            })
            .unwrap_or_else(|| " PCIe Throughput ".to_string());
        with_spark_data_u32(&history.pcie_tx_kbps, |data| {
            let sparkline = Sparkline::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(pcie_title.as_str()),
                )
                .data(data)
                .style(Style::default().fg(Color::LightCyan));
            f.render_widget(sparkline, rows[3]);
        });
    }
}

fn draw_system_charts(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // CPU total sparkline
    let cpu_label = app
        .system_metrics
        .as_ref()
        .map(|s| format!(" CPU Total {:.1}% ", s.cpu_total))
        .unwrap_or_else(|| " CPU Total ".to_string());
    with_spark_data_f32(&app.system_history.cpu_total, |data| {
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(cpu_label.as_str()),
            )
            .data(data)
            .max(100)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(sparkline, rows[0]);
    });

    // RAM sparkline
    let ram_label = app
        .system_metrics
        .as_ref()
        .map(|s| {
            format!(
                " RAM {:.1}/{:.1} GiB ({:.1}%) ",
                s.ram_used_gb(),
                s.ram_total_gb(),
                s.ram_percent()
            )
        })
        .unwrap_or_else(|| " RAM ".to_string());
    with_spark_data_f64(&app.system_history.ram_percent, |data| {
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(ram_label.as_str()),
            )
            .data(data)
            .max(100)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(sparkline, rows[1]);
    });
}

fn vram_pct_color(pct: f64) -> Color {
    if pct > 90.0 {
        Color::Red
    } else if pct > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn temp_color(temp: u32) -> Color {
    if temp > 80 {
        Color::Red
    } else if temp > 60 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn pstate_color(ps: &str) -> Color {
    match ps {
        "P0" => Color::Green,
        "P1" | "P2" | "P3" | "P4" => Color::Yellow,
        _ => Color::Red,
    }
}

fn draw_vram_top_processes(f: &mut Frame, app: &App, area: Rect) {
    let m = match app.selected_metrics() {
        Some(m) => m,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Top Processes ");
            f.render_widget(block, area);
            return;
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Top Processes ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::with_capacity(6);

    // Header row
    lines.push(Line::from(vec![Span::styled(
        format!("{:<7} {:<15} {:>10}", "PID", "Process", "VRAM"),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]));

    if m.top_processes.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No compute processes",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for proc in m.top_processes.iter().take(5) {
            let name: String = proc.name.chars().take(15).collect();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<7}", proc.pid),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{:<15}", name), Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:>7} MB", proc.vram_used_mb()),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }
    }

    let para = Paragraph::new(lines);
    f.render_widget(para, inner);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            " q",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Tab/↑↓",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Switch GPU  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" [{}/{}]", app.selected_gpu + 1, app.metrics.len().max(1)),
            Style::default().fg(Color::Cyan),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}
