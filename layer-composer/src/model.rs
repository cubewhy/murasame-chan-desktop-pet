use std::{collections::BTreeMap, io::Read};

use image::DynamicImage;
use zip::ZipArchive;

use crate::{LayerMetadata, TopLayerMetadata, compose::ComposeError, compose_layers_from_model};

mod json_model {
    use std::collections::HashMap;

    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Root {
        pub layers: HashMap<String, Layer>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    pub enum Layer {
        TopLayer {
            metadata: String,
            #[serde(default)]
            description: Option<String>,
        },
        BaseLayer {
            #[serde(rename = "type")]
            r#type: BaseType,
            offset: [i32; 2],
            description: Option<String>,
        },
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum BaseType {
        BaseLayer,
    }
}

#[derive(Clone, Debug)]
pub struct ModelManifest {
    pub layers: BTreeMap<String, LayerManifest>, // NOTE: we care the order of the layers
}

#[derive(Clone, Debug)]
pub enum LayerManifest {
    BaseLayer {
        offset: [i32; 2],
        description: Option<String>,
    },
    TopLayer {
        description: Option<String>,
        metadata: TopLayerMetadata,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("Invalid model: no manifest.json found")]
    NoManifest,
    #[error("Failed to parse json {0}")]
    JsonParsing(#[from] serde_json::Error),
    #[error("No layer with name {0}: {1}")]
    NoLayer(String, #[source] zip::result::ZipError),
    #[error("Failed to open image")]
    ImageParsing(#[from] image::ImageError),
    #[error("IO error")]
    IOError(#[from] std::io::Error),
}

pub fn parse_model_manifest<T: std::io::Read + std::io::Seek>(
    model_zip: &mut ZipArchive<T>,
) -> Result<ModelManifest, ModelError> {
    // parse manifest json
    let manifest: json_model::Root = {
        // get manifest.json
        let mut manifest_entry = model_zip
            .by_name("manifest.json")
            .map_err(|_err| ModelError::NoManifest)?;
        serde_json::from_reader(&mut manifest_entry)?
    };

    let mut layers: BTreeMap<String, LayerManifest> = BTreeMap::new();

    // read layers
    for (layer_filename, layer_metadata) in manifest.layers.iter() {
        // NOTE: make sure &mut model_zip is dropped
        {
            if model_zip
                .by_name(&format!("layers/{layer_filename}"))
                .is_err()
            {
                // layer image not found
                continue;
            }
        }
        let layer_manifest: LayerManifest = match layer_metadata {
            json_model::Layer::TopLayer {
                metadata,
                description,
            } => {
                let metadata: LayerMetadata = {
                    let Ok(mut entry) = model_zip.by_name(format!("metadata/{metadata}").as_str())
                    else {
                        // skip parse this layer: no metadata found
                        continue;
                    };
                    serde_json::from_reader(&mut entry)?
                };

                LayerManifest::TopLayer {
                    description: description.to_owned(),
                    metadata: metadata.top_layer,
                }
            }
            json_model::Layer::BaseLayer {
                r#type: _,
                offset,
                description,
            } => LayerManifest::BaseLayer {
                offset: offset.clone(),
                description: description.to_owned(),
            },
        };
        layers.insert(layer_filename.to_string(), layer_manifest);
    }
    Ok(ModelManifest { layers })
}

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("No matched layer manifest for layer {0}")]
    NoMatchedLayerManifest(String),
    #[error("Cannot remix multiple base layers")]
    MultipleBaseLayers,
    #[error("No base layer loaded")]
    NoBaseLayerLoaded,
    #[error("Failed to compose: {0}")]
    Compose(#[from] ComposeError),
    #[error("model error")]
    Model(#[from] ModelError),
}

pub struct Model<'a, T>
where
    T: std::io::Read + std::io::Seek,
{
    manifest: ModelManifest,
    zip: &'a mut ZipArchive<T>,
}

impl<'a, T> Model<'a, T>
where
    T: std::io::Read + std::io::Seek,
{
    pub fn from_zip(zip: &'a mut ZipArchive<T>) -> Result<Self, ModelError> {
        let manifest = parse_model_manifest(zip)?;
        Ok(Self { manifest, zip })
    }

    pub fn render(&mut self, layers: &Vec<String>) -> Result<DynamicImage, RenderError> {
        let mut outcome: Option<DynamicImage> = None;
        let mut base_layer_manifest: Option<LayerManifest> = None;
        for layer in layers {
            let layer_manifest = {
                let Some(layer_manifest) = self.manifest.layers.get(layer) else {
                    return Err(RenderError::NoMatchedLayerManifest(layer.to_string()));
                };
                layer_manifest.clone()
            };
            match &layer_manifest {
                LayerManifest::BaseLayer { .. } => {
                    if outcome.is_none() {
                        outcome = Some(self.get_image(layer)?);
                        base_layer_manifest = Some(layer_manifest);
                    } else {
                        return Err(RenderError::MultipleBaseLayers);
                    }
                }
                LayerManifest::TopLayer { .. } => {
                    if let Some(base_layer) = outcome {
                        let top_layer = self.get_image(layer)?;
                        outcome = Some(
                            compose_layers_from_model(
                                &base_layer,
                                &top_layer,
                                base_layer_manifest.as_ref().unwrap(),
                                &layer_manifest,
                            )?
                            .into(),
                        );
                    } else {
                        return Err(RenderError::NoBaseLayerLoaded);
                    }
                }
            }
        }

        Ok(outcome.unwrap())
    }

    pub fn get_image(&mut self, layer_name: impl Into<String>) -> Result<DynamicImage, ModelError> {
        let layer_name = layer_name.into();
        // get the entry
        let mut entry = self
            .zip
            .by_name(&format!("layers/{layer_name}"))
            .map_err(|_err| ModelError::NoLayer(layer_name, _err))?;

        // read to bytes
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;

        // read image
        let image = image::load_from_memory(&mut buf)?;

        Ok(image)
    }
}
