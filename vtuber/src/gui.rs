use bytes::Bytes;
use eframe::egui;
use tokio::sync::broadcast;

use crate::bus::UiEvent;

pub fn run_gui(ui_rx: broadcast::Receiver<UiEvent>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Vtuber App",
        options,
        Box::new(|_cc| Ok(Box::new(VtuberApp::new(ui_rx)))),
    )
}

#[derive(Debug, Default)]
pub struct AppState {
    pub recent_comments: Vec<(String, String)>,
    pub current_line: Option<(String, Vec<String>, Bytes)>,
}

impl AppState {
    pub fn push_comment(&mut self, user: String, text: String) {
        self.recent_comments.push((user, text));
        if self.recent_comments.len() > 50 {
            self.recent_comments.remove(0);
        }
    }
}

#[derive(Debug)]
pub struct VtuberApp {
    state: AppState,
    ui_rx: broadcast::Receiver<UiEvent>,
}

impl VtuberApp {
    pub fn new(ui_rx: broadcast::Receiver<UiEvent>) -> Self {
        Self {
            state: AppState::default(),
            ui_rx,
        }
    }

    fn poll_events(&mut self) {
        loop {
            match self.ui_rx.try_recv() {
                Ok(UiEvent::NewComment(e)) => self.state.push_comment(e.user, e.text),
                Ok(UiEvent::AiReply {
                    text,
                    layers,
                    voice,
                }) => self.state.current_line = Some((text, layers, voice)),
                Ok(_) => {} // TODO: display errors
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }
    }
}

impl eframe::App for VtuberApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();

        // TODO: do render

        egui::Window::new("Vtuber Frontend").show(ctx, |ui| {
            if let Some((line, layers, _voice)) = &self.state.current_line {
                ui.label(format!("text: {}", line));
                ui.label(format!("Layers: {}", layers.join(",")));
            }
            ui.label(format!(
                "total comments: {}",
                self.state.recent_comments.len()
            ));
        });

        ctx.request_repaint();
    }
}
