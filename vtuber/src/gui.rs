use eframe::egui;

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Vtuber App",
        options,
        Box::new(|_cc| Ok(Box::<VtuberApp>::default())),
    )
}

#[derive(Debug)]
pub struct AppState {
    pub recent_comments: Vec<(String, String)>,
    pub current_line: Option<String>,
}

impl AppState {
    pub fn push_comment(&mut self, user: String, text: String) {
        // TODO: nsfw filter
        self.recent_comments.push((user, text));
        if self.recent_comments.len() > 50 {
            self.recent_comments.remove(0);
        }
    }
}

#[derive(Debug)]
pub struct VtuberApp {}

impl Default for VtuberApp {
    fn default() -> Self {
        Self {}
    }
}

impl eframe::App for VtuberApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        todo!()
    }
}
