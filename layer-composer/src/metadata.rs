use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerMetadata {
    #[serde(rename = "base_layer")]
    pub base_layer: BaseLayer,
    #[serde(rename = "top_layer")]
    pub top_layer: TopLayer,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseLayer {
    pub width: u32,
    pub height: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopLayer {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub opacity: f32,
}
