use bytes::Bytes;
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub enum InEvent {
    Comment(CommentEvent),
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    NewComment(CommentEvent),
    AiThinking,
    AiReply {
        text: String,
        layers: Vec<String>,
        voice: Bytes,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct CommentEvent {
    pub user: String,
    pub text: String,
    pub ts_ms: i64,
}

pub struct Bus {
    pub in_tx: mpsc::Sender<InEvent>,
    pub in_rx: mpsc::Receiver<InEvent>,
    pub ui_tx: broadcast::Sender<UiEvent>,
    pub ui_rx: broadcast::Receiver<UiEvent>,
}

impl Bus {
    pub fn new(buffer: usize) -> Self {
        let (in_tx, in_rx) = mpsc::channel(buffer);
        let (ui_tx, ui_rx) = broadcast::channel(buffer);
        Self {
            in_tx,
            in_rx,
            ui_tx,
            ui_rx,
        }
    }
}

pub struct FrontendHandle {
    pub ui_rx: broadcast::Receiver<UiEvent>,
}
