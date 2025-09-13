use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Seek},
    sync::Arc,
};

use image::DynamicImage;
use zip::{ZipArchive, result::ZipError};

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
            #[serde(default)]
            bindings: Vec<String>,
        },
        BaseLayer {
            #[serde(rename = "type")]
            #[allow(unused)]
            r#type: BaseType,
            offset: [i32; 2],
            description: Option<String>,
            #[serde(default)]
            bindings: Vec<String>,
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
    pub layers: BTreeMap<String, LayerManifest>, // we care the order of the layers
}

#[derive(Clone, Debug)]
pub enum LayerManifest {
    BaseLayer {
        offset: [i32; 2],
        description: Option<String>,
        bindings: Vec<String>,
    },
    TopLayer {
        description: Option<String>,
        metadata: TopLayerMetadata,
        bindings: Vec<String>,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("Failed to open zip: {0}")]
    Zip(#[from] ZipError),
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
        if model_zip
            .by_name(&format!("layers/{layer_filename}"))
            .is_err()
        {
            // layer image not found
            continue;
        }
        let layer_manifest: LayerManifest = match layer_metadata {
            json_model::Layer::TopLayer {
                metadata,
                description,
                bindings,
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
                    bindings: bindings.to_owned(),
                }
            }
            json_model::Layer::BaseLayer {
                r#type: _,
                offset,
                description,
                bindings,
            } => LayerManifest::BaseLayer {
                offset: *offset,
                description: description.to_owned(),
                bindings: bindings.to_owned(),
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
    #[error("No layers provided")]
    NoLayersProvided,
}

#[derive(Clone)]
pub struct Model {
    bytes: Arc<Vec<u8>>,
    manifest: Arc<ModelManifest>,
}

pub struct LayerDescription {
    pub name: String,
    pub description: String,
}

impl Model {
    pub fn from_reader<R: Read + Seek>(mut reader: R) -> Result<Self, ModelError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;

        let mut zip = ZipArchive::new(Cursor::new(&bytes[..]))?;
        let manifest = parse_model_manifest(&mut zip)?;

        Ok(Self {
            bytes: Arc::new(bytes),
            manifest: Arc::new(manifest),
        })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ModelError> {
        let mut zip = ZipArchive::new(Cursor::new(&bytes[..]))?;
        let manifest = parse_model_manifest(&mut zip)?;
        Ok(Self {
            bytes: Arc::new(bytes),
            manifest: Arc::new(manifest),
        })
    }

    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ModelError> {
        let file = std::fs::File::open(path)?;
        Self::from_reader(file)
    }

    #[inline]
    fn open_zip(&self) -> Result<ZipArchive<Cursor<&[u8]>>, std::io::Error> {
        Ok(ZipArchive::new(Cursor::new(&self.bytes[..]))?)
    }

    pub fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }

    pub fn layer_descriptions(&self) -> BTreeMap<i32, LayerDescription> {
        let mut map = BTreeMap::new();
        for (i, (layer_name, layer_manifest)) in self.manifest.layers.iter().enumerate() {
            match layer_manifest {
                LayerManifest::TopLayer { description, .. }
                | LayerManifest::BaseLayer { description, .. } => {
                    if let Some(desc) = description {
                        map.insert(
                            i.try_into().unwrap(),
                            LayerDescription {
                                name: layer_name.to_string(),
                                description: desc.to_string(),
                            },
                        );
                    }
                }
            }
        }
        map
    }

    pub fn render(&mut self, layers: &[String]) -> Result<DynamicImage, RenderError> {
        let mut flat: Vec<String> = Vec::with_capacity(layers.len());
        for name in layers {
            {
                let layer_manifest = self
                    .manifest
                    .layers
                    .get(name)
                    .ok_or_else(|| RenderError::NoMatchedLayerManifest(name.clone()))?;

                flat.push(name.clone());

                match layer_manifest {
                    LayerManifest::BaseLayer { bindings, .. }
                    | LayerManifest::TopLayer { bindings, .. } => {
                        flat.extend(bindings.iter().cloned());
                    }
                }
            }
        }

        let mut outcome: Option<DynamicImage> = None;
        let mut base_name: Option<String> = None;

        for name in &flat {
            let is_base = {
                let manifest = self
                    .manifest
                    .layers
                    .get(name)
                    .ok_or_else(|| RenderError::NoMatchedLayerManifest(name.clone()))?;

                matches!(manifest, LayerManifest::BaseLayer { .. })
            };

            if is_base {
                if outcome.is_some() {
                    return Err(RenderError::MultipleBaseLayers);
                }
                let img = self.get_image(name)?;
                outcome = Some(img);
                base_name = Some(name.clone());
            } else {
                let base_img = outcome.as_ref().ok_or(RenderError::NoBaseLayerLoaded)?;

                let top_img = self.get_image(name)?;

                let composed = {
                    let base_key = base_name
                        .as_ref()
                        .expect("base should be set when outcome is Some");

                    let base_manifest = self
                        .manifest
                        .layers
                        .get(base_key)
                        .expect("base manifest must exist");

                    let top_manifest = self
                        .manifest
                        .layers
                        .get(name)
                        .expect("top manifest must exist");

                    compose_layers_from_model(base_img, &top_img, base_manifest, top_manifest)?
                };

                outcome = Some(composed.into());
            }
        }

        outcome.ok_or(RenderError::NoLayersProvided)
    }

    pub fn get_image(&mut self, layer_name: &str) -> Result<DynamicImage, ModelError> {
        // get the entry
        let mut zip = self.open_zip()?;
        let mut entry = zip
            .by_name(&format!("layers/{layer_name}"))
            .map_err(|_err| ModelError::NoLayer(layer_name.to_string(), _err))?;

        // read to bytes
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;

        // read image
        let image = image::load_from_memory(&buf)?;

        Ok(image)
    }
}
