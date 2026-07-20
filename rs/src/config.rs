use std::path::PathBuf;

pub fn app_data_dir() -> PathBuf {
    if let Some(dir) = dirs::data_dir() {
        dir.join("GoRun")
    } else {
        PathBuf::from(".")
    }
}

pub fn config_path() -> PathBuf {
    app_data_dir().join("config.json")
}

pub fn icons_dir() -> PathBuf {
    app_data_dir().join("icons")
}
