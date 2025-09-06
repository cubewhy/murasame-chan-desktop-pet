use eframe::egui;

use crate::gui::FrontendApp;

mod gui;

pub fn run() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    env_logger::init(); // TODO: add default log level

    // init gui
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 240.0])
            .with_transparent(true)
            .with_window_level(egui::WindowLevel::AlwaysOnTop),
        ..Default::default()
    };

    eframe::run_native(
        "Murasame-chan",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<FrontendApp>::default())
        }),
    )
    .map_err(|err| anyhow::anyhow!("Failed to init egui: {err}"))?;

    Ok(())
}
