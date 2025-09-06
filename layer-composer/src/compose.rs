use image::{DynamicImage, ImageBuffer, Rgba, imageops};

use crate::LayerMetadata;

pub fn compose_layers(
    base_layer: &DynamicImage,
    top_layer: &DynamicImage,
    metadata: &LayerMetadata,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    // load images with alpha channel
    let mut base = base_layer.to_rgba8();
    let mut top = top_layer.to_rgba8();

    top = imageops::resize(
        &top,
        metadata.top_layer.scaled_width,
        metadata.top_layer.scaled_height,
        imageops::FilterType::Lanczos3,
    );

    // Apply opacity
    if metadata.top_layer.opacity < 1.0 {
        for pixel in top.pixels_mut() {
            // get the alpha channel
            let a = pixel[3] as f32 / 255.0 * metadata.top_layer.opacity;
            // mut the alpha chan
            pixel[3] = (a * 255.0).round() as u8
        }
    }

    // overlay the top layer on the base layer
    imageops::overlay(
        &mut base,
        &top,
        metadata.top_layer.x.into(),
        metadata.top_layer.y.into(),
    );

    base
}
