use std::{
    collections::HashMap,
    io::{BufReader, Cursor},
    sync::mpsc,
};

use bytes::Bytes;
use eframe::egui::{self, Image};
use rodio::{Decoder, OutputStream, OutputStreamBuilder};
use tokio::sync::broadcast;

use crate::{
    bus::UiEvent,
    config::{AppConfig, RenderConfig},
};

pub fn run_gui(
    ui_rx: broadcast::Receiver<UiEvent>,
    app_config: &AppConfig,
) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Vtuber App",
        options,
        Box::new(|_cc| Ok(Box::new(VtuberApp::new(ui_rx, app_config)))),
    )
}

#[derive(Default)]
pub struct AppState {
    pub recent_comments: Vec<(String, String)>,
    pub current_line: Option<(String, Vec<String>, Bytes)>,
    pub layer_textures: HashMap<String, egui::TextureHandle>,
}

impl AppState {
    pub fn push_comment(&mut self, user: String, text: String) {
        self.recent_comments.push((user, text));
        if self.recent_comments.len() > 50 {
            self.recent_comments.remove(0);
        }
    }
}

pub struct VtuberApp {
    state: AppState,
    ui_rx: broadcast::Receiver<UiEvent>,

    composite_tex: Option<egui::TextureHandle>,

    img_rx: mpsc::Receiver<egui::ColorImage>,
    img_tx: mpsc::Sender<egui::ColorImage>,

    audio_stream: OutputStream,

    render_config: RenderConfig,
}

impl VtuberApp {
    pub fn new(ui_rx: broadcast::Receiver<UiEvent>, app_config: &AppConfig) -> Self {
        let (img_tx, img_rx) = mpsc::channel::<egui::ColorImage>();
        let audio_stream = OutputStreamBuilder::open_default_stream().unwrap();
        Self {
            state: AppState::default(),
            composite_tex: None,
            ui_rx,
            img_rx,
            img_tx,
            audio_stream,

            render_config: app_config.render.to_owned(),
        }
    }

    fn drain_pending_image(&mut self, ctx: &egui::Context) {
        while let Ok(ci) = self.img_rx.try_recv() {
            let tex = ctx.load_texture("composited", ci, egui::TextureOptions::LINEAR);
            self.composite_tex = Some(tex);
        }
    }

    fn poll_events(&mut self, ctx: &egui::Context) {
        loop {
            match self.ui_rx.try_recv() {
                Ok(UiEvent::NewComment(e)) => {
                    self.state.push_comment(e.user, e.text);
                }

                Ok(UiEvent::AiReply {
                    text,
                    layers: reply_layers,
                    voice,
                }) => {
                    self.state.current_line =
                        Some((text.clone(), reply_layers.clone(), voice.clone()));

                    let mut layers_to_render = Vec::with_capacity(1 + reply_layers.len());
                    layers_to_render.push(self.render_config.base_layer.clone());
                    layers_to_render.extend(reply_layers.clone());

                    let mut model = self.render_config.model.clone();

                    let tx = self.img_tx.clone();

                    std::thread::spawn(move || {
                        let image = model
                            .render(&layers_to_render)
                            .expect("image render failed");
                        let color_image = rgba_image_to_color_image(&image.into());
                        let _ = tx.send(color_image);
                    });

                    let mixer_handle = self.audio_stream.mixer().clone();
                    let voice_bytes = voice.to_vec();

                    std::thread::spawn(move || {
                        let cursor = std::io::Cursor::new(voice_bytes);
                        let reader = std::io::BufReader::new(cursor);
                        if let Ok(source) = rodio::Decoder::new(reader) {
                            mixer_handle.add(source);
                        }
                    });

                    ctx.request_repaint();
                }

                Ok(_) => {
                    // TODO: display errors
                }

                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }
    }
}

impl eframe::App for VtuberApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events(ctx);
        self.drain_pending_image(ctx);

        // do render
        egui::CentralPanel::default().show(ctx, |ui| {
            // display the image
            if let Some(tex) = &self.composite_tex {
                ui.add(Image::new(tex));
            } else {
                ui.label("(compositing layers…)");
            }

            // display text
            // TODO: display text on the image
            if let Some((line, layers, _voice)) = &self.state.current_line {
                ui.label(format!("text: {}", line));
                ui.label(format!("Layers: {}", layers.join(",")));
            }
        });

        ctx.request_repaint();
    }
}

fn rgba_image_to_color_image(img: &image::RgbaImage) -> egui::ColorImage {
    let (w, h) = img.dimensions();
    let raw = img.as_raw();
    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], raw)
}
