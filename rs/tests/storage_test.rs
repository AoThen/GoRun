#[cfg(test)]
mod tests {
    use GoRun::hotkey::parse_hotkey_string;
    use GoRun::model::AppConfig;
    use GoRun::model::{generate_id, Item, ViewType};
    use GoRun::runner::is_url;
    use GoRun::storage::Storage;

    #[test]
    fn test_is_url_http() {
        assert!(is_url("http://example.com"));
        assert!(is_url("https://example.com"));
        assert!(is_url("ftp://files.example.com"));
        assert!(is_url("steam://run/123"));
        assert!(is_url("mailto:test@example.com"));
    }

    #[test]
    fn test_is_url_not_url() {
        assert!(!is_url("C:\\Windows\\notepad.exe"));
        assert!(!is_url("D:\\Games\\game.exe"));
        assert!(!is_url("relative/path/file.txt"));
    }

    #[test]
    fn test_generate_id_format() {
        let id = generate_id("cat");
        assert!(id.starts_with("cat_"));
        assert!(id.len() > 4);
    }

    #[test]
    fn test_hotkey_parse() {
        let hk = parse_hotkey_string("Ctrl+Alt+M").unwrap();
        assert_eq!(hk.modifiers, 0x0003);
        assert_eq!(hk.vk, 'M' as u32);
    }

    #[test]
    fn test_storage_roundtrip() {
        let mut config = AppConfig::default();
        config.version = "1.0.0".to_string();
        config.data.items.push(Item {
            id: "item_test_1".to_string(),
            name: "Notepad".to_string(),
            target: "C:\\Windows\\notepad.exe".to_string(),
            ..Default::default()
        });

        let path = std::env::temp_dir()
            .join("gorun_rs_test.json")
            .to_str()
            .unwrap()
            .to_string();
        let storage = Storage::new(path.clone());
        assert!(storage.save(&config));

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.data.items.len(), 1);
        assert_eq!(loaded.data.items[0].name, "Notepad");

        let _ = std::fs::remove_file(&path);
    }
}
