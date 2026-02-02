use std::io::{self, stdout};
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use crate::{
    start_headless_internal, start_miner_internal, start_node_internal,
    start_tx_mining_internal, stop_headless_internal, stop_miner_internal,
    stop_node_internal, stop_tx_mining_internal, SharedState,
};

/// Snapshot of dashboard state for rendering.
struct DashboardState {
    node_running: bool,
    miner_running: bool,
    headless_running: bool,
    tx_mining_running: bool,
    block_height: Option<u64>,
    fullnode_url: String,
    wallet_url: String,
    tx_mining_url: String,
    mcp_port: u16,
}

/// RAII guard to restore terminal on drop (including panics).
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

/// Run the interactive TUI dashboard. Returns when the user presses 'q'.
pub async fn run_tui(
    state: SharedState,
    mcp_port: u16,
    fullnode_url: String,
    wallet_url: String,
    tx_mining_url: String,
) -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Block height poller
    let block_height: Arc<std::sync::Mutex<Option<u64>>> =
        Arc::new(std::sync::Mutex::new(None));
    let bh = block_height.clone();
    let poll_url = fullnode_url.clone();
    let _poller = tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            if let Ok(resp) = client
                .get(format!("{}/v1a/status/", poll_url))
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(h) = json.get("dag").and_then(|d| d.get("best_block_height")).and_then(|v| v.as_u64()) {
                        *bh.lock().unwrap() = Some(h);
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    let tick_rate = Duration::from_millis(500);

    loop {
        // Build snapshot
        let dash = {
            let s = state.lock().await;
            DashboardState {
                node_running: s.node_running,
                miner_running: s.miner_running,
                headless_running: s.headless_running,
                tx_mining_running: s.tx_mining_running,
                block_height: *block_height.lock().unwrap(),
                fullnode_url: fullnode_url.clone(),
                wallet_url: wallet_url.clone(),
                tx_mining_url: tx_mining_url.clone(),
                mcp_port,
            }
        };

        terminal.draw(|f| draw(f, &dash))?;

        // Poll for key events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('s') => {
                        let st = state.clone();
                        tokio::spawn(async move {
                            let _ = start_node_internal(&st).await;
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            let _ = start_tx_mining_internal(&st).await;
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            let _ = start_miner_internal(&st, None).await;
                            let _ = start_headless_internal(&st).await;
                        });
                    }
                    KeyCode::Char('x') => {
                        let st = state.clone();
                        tokio::spawn(async move {
                            let _ = stop_miner_internal(&st).await;
                            let _ = stop_headless_internal(&st).await;
                            let _ = stop_tx_mining_internal(&st).await;
                            let _ = stop_node_internal(&st).await;
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn status_span(running: bool) -> Span<'static> {
    if running {
        Span::styled("RUNNING", Style::default().fg(Color::Green).bold())
    } else {
        Span::styled("STOPPED", Style::default().fg(Color::Red).bold())
    }
}

fn draw(f: &mut Frame, dash: &DashboardState) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(8),   // table
            Constraint::Length(3), // block height
            Constraint::Length(2), // footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("Hathor Forge CLI v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::Cyan).bold(),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    f.render_widget(header, chunks[0]);

    // Service table
    let rows = vec![
        Row::new(vec![
            Cell::from("Fullnode"),
            Cell::from(status_span(dash.node_running)),
            Cell::from(dash.fullnode_url.as_str().to_string()),
        ]),
        Row::new(vec![
            Cell::from("Miner"),
            Cell::from(status_span(dash.miner_running)),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from("Wallet-Headless"),
            Cell::from(status_span(dash.headless_running)),
            Cell::from(dash.wallet_url.as_str().to_string()),
        ]),
        Row::new(vec![
            Cell::from("Tx-Mining"),
            Cell::from(status_span(dash.tx_mining_running)),
            Cell::from(dash.tx_mining_url.as_str().to_string()),
        ]),
        Row::new(vec![
            Cell::from("MCP Server"),
            Cell::from(status_span(true)),
            Cell::from(format!("http://127.0.0.1:{}", dash.mcp_port)),
        ]),
    ];

    let widths = [
        Constraint::Length(18),
        Constraint::Length(10),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Service", "Status", "Details"])
                .style(Style::default().bold().fg(Color::White))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(table, chunks[1]);

    // Block height
    let bh_text = match dash.block_height {
        Some(h) => format!("Block Height: {}", h),
        None if dash.node_running => "Block Height: ...".to_string(),
        None => "Block Height: -".to_string(),
    };
    let bh = Paragraph::new(bh_text).style(Style::default().fg(Color::Yellow));
    f.render_widget(bh, chunks[2]);

    // Footer keybindings
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("q", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" quit │ "),
        Span::styled("s", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" start all │ "),
        Span::styled("x", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" stop all"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}
