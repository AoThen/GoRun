use std::path::PathBuf;

pub fn app_data_dir() -> PathBuf {
    if let Some(dir) = dirs::data_dir() {
        let path = dir.join("GoRun");
        log::debug!("App data dir: {:?}", path);
        path
    } else {
        PathBuf::from(".")
    }
}

pub fn config_path() -> PathBuf {
    let path = app_data_dir().join("config.json");
    log::debug!("Config path: {:?}", path);
    path
}

pub fn icons_dir() -> PathBuf {
    app_data_dir().join("icons")
}
