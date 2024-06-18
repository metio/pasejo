use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Configuration {
    pub stores: Vec<Store>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Store {
    pub path: String,
    pub alias: String,
    pub r#type: StoreTypes,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, clap::ValueEnum)]
pub enum StoreTypes {
    #[default]
    Local,
    Git,
}

impl StoreTypes {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoreTypes::Local => "local",
            StoreTypes::Git => "git",
        }
    }
}
