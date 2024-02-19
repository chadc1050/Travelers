use std::{collections::HashMap, io::ErrorKind};

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    utils::BoxedFuture,
};

use serde::Deserialize;

// TODO: Have the hashmap deserialize to a u8 key instead of a string.
#[derive(Asset, Clone, Debug, TypePath, Deserialize)]
pub struct SchematicAsset {
    #[serde(flatten)]
    pub tiles: HashMap<String, TileSchematic>,
}

#[derive(Resource)]
pub struct SchematicResource(pub Handle<SchematicAsset>);

#[derive(Clone, Debug, Deserialize)]
pub struct TileSchematic {
    pub name: String,
    pub sheet: String,
    pub weight: u8,
    #[serde(rename = "0")]
    pub north: Vec<u8>,
    #[serde(rename = "1")]
    pub east: Vec<u8>,
    #[serde(rename = "2")]
    pub south: Vec<u8>,
    #[serde(rename = "3")]
    pub west: Vec<u8>,
}

#[derive(Default)]
pub struct SchematicLoader;

impl AssetLoader for SchematicLoader {
    type Asset = SchematicAsset;

    type Settings = ();

    type Error = std::io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _: &'a Self::Settings,
        _: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            _ = reader.read_to_end(&mut bytes).await;
            let serialized = serde_json::from_slice::<SchematicAsset>(&bytes);

            match serialized {
                Ok(data) => {
                    info!("Successfully loaded asset");
                    Ok(data)
                }
                Err(err) => Err(Self::Error::new(
                    ErrorKind::InvalidData,
                    format!("Failed to deserialize Json File! Err {err}"),
                )),
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
