use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;

/// Ring buffer for service log lines, shared across all service processes.
const LOG_BUFFER_CAPACITY: usize = 1000;

#[derive(Clone)]
pub struct LogBuffer(Arc<std::sync::Mutex<VecDeque<String>>>);

impl LogBuffer {
    pub fn new() -> Self {
        Self(Arc::new(std::sync::Mutex::new(VecDeque::with_capacity(
            LOG_BUFFER_CAPACITY,
        ))))
    }

    pub fn push(&self, line: String) {
        let mut buf = self.0.lock().unwrap();
        if buf.len() == LOG_BUFFER_CAPACITY {
            buf.pop_front();
        }
        buf.push_back(line);
    }

    pub fn lines(&self) -> Vec<String> {
        self.0.lock().unwrap().iter().cloned().collect()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn a background task that reads lines from an async reader and pushes them
/// into the shared log buffer with a `[prefix]` tag.
pub fn spawn_log_reader<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    reader: R,
    log_buf: LogBuffer,
    prefix: &'static str,
) {
    tokio::spawn(async move {
        let br = tokio::io::BufReader::new(reader);
        let mut lines = br.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log_buf.push(format!("[{}] {}", prefix, line));
        }
    });
}

pub struct AppState {
    pub node_running: bool,
    pub miner_running: bool,
    pub explorer_server_running: bool,
    pub headless_running: bool,
    pub tx_mining_running: bool,
    pub node_child_id: Option<u32>,
    pub miner_child_id: Option<u32>,
    pub headless_child_id: Option<u32>,
    pub tx_mining_child_id: Option<u32>,
    pub explorer_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub data_dir: Option<String>,
    pub log_buffer: LogBuffer,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            node_running: false,
            miner_running: false,
            explorer_server_running: false,
            headless_running: false,
            tx_mining_running: false,
            node_child_id: None,
            miner_child_id: None,
            headless_child_id: None,
            tx_mining_child_id: None,
            explorer_shutdown: None,
            data_dir: None,
            log_buffer: LogBuffer::new(),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
