use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::Read,
    sync::{Arc, mpsc},
};

use bytes::Bytes;
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Image};
use font_kit::{
    family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
};
use rodio::{OutputStream, OutputStreamBuilder, Source};
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
    need_init: bool,

    state: AppState,

    character_name: String,

    ui_rx: broadcast::Receiver<UiEvent>,

    composite_tex: Option<egui::TextureHandle>,

    img_rx: mpsc::Receiver<egui::ColorImage>,
    img_tx: mpsc::Sender<egui::ColorImage>,

    audio_stream: OutputStream,
    pending: VecDeque<(String, Vec<String>, Bytes)>,
    is_playing: bool,
    finished_rx: mpsc::Receiver<()>,
    finished_tx: mpsc::Sender<()>,

    render_config: RenderConfig,
}

impl VtuberApp {
    pub fn new(ui_rx: broadcast::Receiver<UiEvent>, app_config: &AppConfig) -> Self {
        let (img_tx, img_rx) = mpsc::channel::<egui::ColorImage>();
        let audio_stream = OutputStreamBuilder::open_default_stream().unwrap();
        let (finished_tx, finished_rx) = mpsc::channel();

        Self {
            need_init: true,
            state: AppState::default(),
            character_name: app_config.ai.character_name.to_owned(),
            composite_tex: None,
            ui_rx,
            img_rx,
            img_tx,
            audio_stream,

            pending: VecDeque::new(),
            is_playing: false,
            finished_rx,
            finished_tx,

            render_config: app_config.render.to_owned(),
        }
    }

    fn drain_pending_image(&mut self, ctx: &egui::Context) {
        while let Ok(ci) = self.img_rx.try_recv() {
            let tex = ctx.load_texture("composited", ci, egui::TextureOptions::LINEAR);
            self.composite_tex = Some(tex);
        }
    }

    fn start_next_if_any(&mut self, ctx: &egui::Context) {
        if let Some((text, reply_layers, voice)) = self.pending.pop_front() {
            self.is_playing = true;

            self.state.current_line = Some((text.clone(), reply_layers.clone(), voice.clone()));

            let mut model = self.render_config.model.clone();
            let mut layers_to_render = Vec::with_capacity(1 + reply_layers.len());
            layers_to_render.push(self.render_config.base_layer.clone());
            layers_to_render.extend(reply_layers.clone());

            let tx_img = self.img_tx.clone();
            std::thread::spawn(move || {
                let image = model
                    .render(&layers_to_render)
                    .expect("image render failed");
                let color_image = rgba_image_to_color_image(&image.into());
                let _ = tx_img.send(color_image);
            });

            let voice_bytes_for_len = voice.clone();
            let voice_bytes_for_play = voice.clone();

            let finished_tx = self.finished_tx.clone();
            let mix_handle = self.audio_stream.mixer().clone();

            std::thread::spawn(move || {
                let total = {
                    let r = std::io::BufReader::new(std::io::Cursor::new(voice_bytes_for_len));
                    rodio::Decoder::new(r).ok().and_then(|s| s.total_duration())
                };

                {
                    let r = std::io::BufReader::new(std::io::Cursor::new(voice_bytes_for_play));
                    if let Ok(source) = rodio::Decoder::new(r) {
                        mix_handle.add(source);
                    }
                }

                if let Some(d) = total {
                    std::thread::sleep(d);
                } else {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }

                let _ = finished_tx.send(());
            });

            ctx.request_repaint();
        }
    }

    fn draw_overlay_lines(
        &self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        lines: &[&str],
        font_id: egui::FontId,
        text_color: egui::Color32,
        padding: egui::Vec2,
        corner_radius: f32,
        bg_color: egui::Color32,
        max_width_override: Option<f32>,
    ) {
        let painter = ui.painter_at(area);

        let max_width =
            max_width_override.unwrap_or_else(|| (area.width() - 2.0 * padding.x).max(0.0));

        let mut galleys: Vec<Arc<egui::Galley>> = Vec::with_capacity(lines.len());
        let mut total_h = 0.0f32;
        let mut max_w = 0.0f32;

        ui.fonts(|f| {
            for &line in lines {
                if line.is_empty() {
                    let galley =
                        f.layout(" ".to_owned(), font_id.clone(), Color32::WHITE, max_width);
                    max_w = max_w.max(galley.size().x);
                    total_h += galley.size().y;
                    galleys.push(galley);
                    continue;
                }
                let galley = f.layout(line.to_owned(), font_id.clone(), Color32::WHITE, max_width);
                max_w = max_w.max(galley.size().x);
                total_h += galley.size().y;
                galleys.push(galley);
            }
        });

        let text_origin = egui::pos2(
            area.left() + padding.x,
            (area.bottom() * 3.0 / 4.0) - padding.y - total_h,
        );

        let bg_rect = egui::Rect::from_min_size(
            egui::pos2(text_origin.x - padding.x, text_origin.y - padding.y),
            egui::vec2(max_w + padding.x * 2.0, total_h + padding.y * 2.0),
        );
        painter.rect(
            bg_rect,
            corner_radius,
            bg_color,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );

        let mut cursor = text_origin;
        for galley in galleys {
            painter.galley(cursor, galley.clone(), text_color);
            cursor.y += galley.size().y;
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
                    self.pending.push_back((text, reply_layers, voice));
                }

                Ok(_) => { /* TODO: display errors */ }

                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }

        if !self.is_playing {
            self.start_next_if_any(ctx);
        }

        while self.finished_rx.try_recv().is_ok() {
            self.is_playing = false;
            self.start_next_if_any(ctx);
        }
    }
}

/// Attempt to load a system font by any of the given `family_names`, returning the first match.
fn load_font_family(family_names: &[&str]) -> Option<Vec<u8>> {
    let system_source = SystemSource::new();

    for &name in family_names {
        match system_source
            .select_best_match(&[FamilyName::Title(name.to_string())], &Properties::new())
        {
            Ok(h) => match &h {
                Handle::Memory { bytes, .. } => {
                    log::debug!("Loaded {name} from memory.");
                    return Some(bytes.to_vec());
                }
                Handle::Path { path, .. } => {
                    log::info!("Loaded {name} from path: {:?}", path);
                    let mut buf = Vec::new();
                    File::open(path).unwrap().read_to_end(&mut buf).unwrap();
                    return Some(buf);
                }
            },
            Err(e) => log::error!("Could not load {}: {:?}", name, e),
        }
    }

    None
}

pub fn load_system_fonts(mut fonts: FontDefinitions) -> FontDefinitions {
    let mut fontdb = HashMap::new();

    fontdb.insert(
        "simplified_chinese",
        vec![
            "Heiti SC",
            "Songti SC",
            "Noto Sans CJK SC", // Good coverage for Simplified Chinese
            "Noto Sans SC",
            "WenQuanYi Zen Hei", // INcludes both Simplified and Traditional Chinese.
            "SimSun",
            "Noto Sans SC",
            "PingFang SC",
            "Source Han Sans CN",
        ],
    );

    fontdb.insert("korean", vec!["Source Han Sans KR"]);

    fontdb.insert(
        "arabic_fonts",
        vec![
            "Noto Sans Arabic",
            "Amiri",
            "Lateef",
            "Al Tarikh",
            "Segoe UI",
        ],
    );

    // Add more stuff here for better language support
    for (region, font_names) in fontdb {
        if let Some(font_data) = load_font_family(&font_names) {
            log::info!("Inserting font {region}");
            fonts
                .font_data
                .insert(region.to_owned(), FontData::from_owned(font_data).into());

            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .push(region.to_owned());
        }
    }
    fonts
}

impl eframe::App for VtuberApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events(ctx);
        self.drain_pending_image(ctx);

        if self.need_init {
            ctx.set_fonts(load_system_fonts(FontDefinitions::empty()));
            self.need_init = false;
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                if let Some(tex) = &self.composite_tex {
                    // Render the image
                    let _image_response =
                        ui.add(Image::new(tex).fit_to_exact_size(ui.available_size_before_wrap()));

                    // Render text
                    if let Some((line, _, _)) = &self.state.current_line {
                        let lines: [&str; 2] = [&format!("【{}】", self.character_name), line];

                        let area = ui.clip_rect();

                        self.draw_overlay_lines(
                            ui,
                            area,
                            &lines,
                            egui::FontId::proportional(26.0),
                            Color32::WHITE,
                            egui::vec2(12.0, 10.0),
                            10.0,
                            Color32::from_black_alpha(160),
                            None,
                        );
                    }
                } else {
                    // TODO: render default image
                    ui.label("(wait for response...)");
                }
            });

        ctx.request_repaint();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

fn rgba_image_to_color_image(img: &image::RgbaImage) -> egui::ColorImage {
    let (w, h) = img.dimensions();
    let raw = img.as_raw();
    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], raw)
}
