use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerMetadata {
    #[serde(rename = "top_layer")]
    pub top_layer: TopLayerMetadata,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopLayerMetadata {
    pub x: i32,
    pub y: i32,
    pub origin_width: u32,
    pub origin_height: u32,
    pub scaled_width: u32,
    pub scaled_height: u32,
    pub scale: f64,
    pub opacity: f32,
}
