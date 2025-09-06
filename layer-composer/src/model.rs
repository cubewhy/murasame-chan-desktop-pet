use std::collections::{BTreeMap, HashMap};

use zip::ZipArchive;

use crate::TopLayerMetadata;

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

pub struct ModelManifest {
    pub layers: BTreeMap<String, LayerManifest>, // NOTE: we care the order of the layers
}

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
}

pub fn parse_model<T: std::io::Read + std::io::Seek>(
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
                let metadata: TopLayerMetadata = {
                    let Ok(mut entry) = model_zip.by_name(format!("metadata/{metadata}").as_str())
                    else {
                        // skip parse this layer: no metadata found
                        continue;
                    };
                    serde_json::from_reader(&mut entry)?
                };

                LayerManifest::TopLayer {
                    description: description.to_owned(),
                    metadata,
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
