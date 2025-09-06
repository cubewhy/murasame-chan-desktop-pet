use std::{fs::File, path::PathBuf};

use eframe::egui::{self, Color32, ColorImage, Image, TextureHandle};
use image::DynamicImage;

#[derive(Debug, Default)]
pub struct FrontendApp {
    input_text: String,
    image: Option<ColorImage>,
}

impl eframe::App for FrontendApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                if let Some(image) = &self.image {
                    let texture: TextureHandle =
                        ctx.load_texture("final_image", image.clone(), Default::default());

                    let new_size = ui.available_size();
                    let image = Image::new(&texture).fit_to_exact_size(new_size);
                    ui.add(image);
                } else {
                    // render default image
                    self.render_image_with_layers(vec!["0_1950", "0_1455", "0_1959"]);
                }
                if ui.button("Render 0").clicked() {
                    self.render_image_with_layers(vec!["0_1950", "0_1455", "0_1959"]);
                }
                if ui.button("Render 1").clicked() {
                    self.render_image_with_layers(vec!["0_1956", "0_1455", "0_1959"])
                }

                if ui.button("Render 2").clicked() {
                    self.render_image_with_layers(vec!["0_1957", "0_1455", "0_1959"])
                }
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

impl FrontendApp {
    fn render_image_with_layers(&mut self, layers: Vec<impl Into<String>>) {
        // TODO: allow customize in .json file
        let layers = layers
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<String>>();

        let mut final_image: Option<DynamicImage> = None;

        for layer_name in layers.into_iter() {
            let layer_path = self.get_layer_path(&layer_name);
            let layer = image::open(layer_path).unwrap();
            if let Some(prev_layer) = final_image {
                // parse metadata
                let metadata = serde_json::from_reader(
                    File::open(self.get_layer_metadata_path(&layer_name)).unwrap(),
                )
                .unwrap();
                // render the layer
                final_image =
                    Some(layer_composer::compose_layers(&prev_layer, &layer, &metadata).into());
            } else {
                // use the first layer as the base image
                final_image = Some(layer);
            }
        }
        let rgba = final_image.unwrap().to_rgba8();
        let (w, h) = rgba.dimensions();
        let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        self.image = Some(color_image);
    }

    fn get_layer_metadata_path(&self, layer_name: impl Into<String>) -> PathBuf {
        let layer_name = layer_name.into();
        let mut layer_path = PathBuf::new();
        layer_path.push("data");
        layer_path.push("metadata");
        layer_path.push(format!("{layer_name}.json"));

        layer_path
    }

    fn get_layer_path(&self, layer_name: impl Into<String>) -> PathBuf {
        let layer_name = layer_name.into();
        let mut layer_path = PathBuf::new();
        layer_path.push("data");
        layer_path.push("layers");
        layer_path.push(format!("ムラサメa_{layer_name}.png"));

        layer_path
    }
}
