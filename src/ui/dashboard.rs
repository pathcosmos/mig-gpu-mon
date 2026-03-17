use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline},
    Frame,
};

use crate::app::App;

// Reusable scratch buffer for sparkline u64 conversion.
// Avoids allocation per draw call. Thread-local since draw is single-threaded.
thread_local! {
    static SPARK_BUF: std::cell::RefCell<Vec<u64>> = std::cell::RefCell::new(Vec::with_capacity(300));
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
    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" mig-gpu-mon "),
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
        .constraints([Constraint::Min(4), Constraint::Length(5)])
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

    let cores = &sys.cpu_usage;
    let half = (cores.len() + 1) / 2;
    let col_width = inner.width / 2;
    let bar_width = col_width.saturating_sub(8) as usize;

    let max_rows = inner.height as usize;
    let mut lines: Vec<Line> = Vec::with_capacity(max_rows);

    for row in 0..max_rows.min(half) {
        let mut spans = Vec::with_capacity(7);

        // Left core
        let left_idx = row;
        if left_idx < cores.len() {
            let usage = cores[left_idx];
            let color = cpu_color(usage);
            spans.push(Span::styled(
                format!("{:>3}", left_idx),
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

        // Right core
        let right_idx = row + half;
        if right_idx < cores.len() {
            let usage = cores[right_idx];
            let color = cpu_color(usage);
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("{:>3}", right_idx),
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
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
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
            Constraint::Percentage(25),
            Constraint::Percentage(40),
            Constraint::Percentage(35),
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
            ListItem::new(format!(
                "{} {} {}: {} | GPU:{}% MEM:{}%",
                indicator, prefix, m.index, m.name, m.gpu_util, m.memory_util
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

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&m.name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("UUID: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &m.uuid[..m.uuid.len().min(20)],
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "VRAM ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} MB", m.memory_used_mb()),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" / {} MB ", m.memory_total_mb()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("({:.1}%)", m.memory_percent()),
                Style::default().fg(vram_pct_color(m.memory_percent())),
            ),
        ]),
        Line::from(vec![
            Span::styled("GPU Util: ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("{}%", m.gpu_util),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Mem Util: ", Style::default().fg(Color::Blue)),
            Span::styled(
                format!("{}%", m.memory_util),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    if let Some(sm) = m.sm_util {
        lines.push(Line::from(vec![
            Span::styled("SM Util:  ", Style::default().fg(Color::Magenta)),
            Span::styled(format!("{}%", sm), Style::default().fg(Color::White)),
        ]));
    }

    if let Some(temp) = m.temperature {
        let temp_color = if temp > 80 {
            Color::Red
        } else if temp > 60 {
            Color::Yellow
        } else {
            Color::Green
        };
        lines.push(Line::from(vec![
            Span::styled("Temp: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}°C", temp), Style::default().fg(temp_color)),
        ]));
    }

    if let (Some(usage), Some(limit)) = (m.power_usage_w(), m.power_limit_w()) {
        lines.push(Line::from(vec![
            Span::styled("Power: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1}W / {:.1}W", usage, limit),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }

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

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ])
        .split(area);

    // GPU Utilization sparkline
    with_spark_data_u32(&history.gpu_util, |data| {
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" GPU Utilization % "),
            )
            .data(data)
            .max(100)
            .style(Style::default().fg(Color::Green));
        f.render_widget(sparkline, rows[0]);
    });

    // Memory Utilization sparkline
    with_spark_data_u32(&history.memory_util, |data| {
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Memory Utilization % "),
            )
            .data(data)
            .max(100)
            .style(Style::default().fg(Color::Blue));
        f.render_widget(sparkline, rows[1]);
    });

    // Gauges row
    if let Some(m) = app.selected_metrics() {
        let gauge_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[2]);

        let gpu_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" GPU % "))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(m.gpu_util.min(100) as u16)
            .label(format!("{}%", m.gpu_util));
        f.render_widget(gpu_gauge, gauge_cols[0]);

        let vram_pct = m.memory_percent().min(100.0) as u16;
        let vram_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" VRAM Usage "))
            .gauge_style(Style::default().fg(Color::Magenta))
            .percent(vram_pct)
            .label(format!(
                "VRAM {} / {} MB ({:.1}%)",
                m.memory_used_mb(),
                m.memory_total_mb(),
                m.memory_percent()
            ));
        f.render_widget(vram_gauge, gauge_cols[1]);
    }
}

fn draw_system_charts(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(45),
            Constraint::Min(3),
        ])
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

    // RAM gauge
    if let Some(sys) = &app.system_metrics {
        let ram_pct = sys.ram_percent().min(100.0) as u16;
        let ram_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" RAM "))
            .gauge_style(Style::default().fg(Color::Yellow))
            .percent(ram_pct)
            .label(format!(
                "{:.1}/{:.1} GiB",
                sys.ram_used_gb(),
                sys.ram_total_gb()
            ));
        f.render_widget(ram_gauge, rows[2]);
    }
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

fn draw_vram_top_processes(f: &mut Frame, app: &App, area: Rect) {
    let m = match app.selected_metrics() {
        Some(m) => m,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" VRAM Top Processes ");
            f.render_widget(block, area);
            return;
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" VRAM Top 5 Processes ");
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
