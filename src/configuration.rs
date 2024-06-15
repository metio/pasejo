use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    stores: Vec<Store>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Store {
    path: String,
    alias: String,
}

impl Default for Configuration {
    fn default() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("wtf.metio.pasejo", "metio.wtf",  "pasejo") {
            let data_directory = proj_dirs.data_dir();
            let default_store_path = data_directory.join("default");

            Self {
                stores: vec![Store {
                    path: String::from(default_store_path.to_string_lossy()),
                    alias: String::from("default"),
                }],
            }
        } else {
            Self {
                stores: vec![],
            }
        }
    }
}
