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
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let portable = exe_dir.join("config.json");
            if portable.exists() {
                log::debug!("Using portable config: {:?}", portable);
                return portable;
            }
        }
    }
    let path = app_data_dir().join("config.json");
    log::debug!("Config path: {:?}", path);
    path
}

pub fn icons_dir() -> PathBuf {
    app_data_dir().join("icons")
}
