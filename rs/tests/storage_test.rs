#[cfg(test)]
mod tests {
    use GoRun::hotkey::parse_hotkey_string;
    use GoRun::model::AppConfig;
    use GoRun::model::{generate_id, Category, Item, ViewType};
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
    fn test_generate_id_unique_within_same_ms() {
        let a = generate_id("item");
        let b = generate_id("item");
        let c = generate_id("item");
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn test_storage_full_roundtrip() {
        let mut config = AppConfig::default();
        config.version = "1.0.0".to_string();
        config.config.window_x = 200;
        config.config.window_y = 300;
        config.config.window_width = 1024;
        config.config.window_height = 768;
        config.config.global_hotkey = "Ctrl+Alt+P".to_string();
        config
            .extra_config
            .insert("theme".to_string(), "light".to_string());
        config.data.categories.push(Category {
            id: "cat_x".to_string(),
            name: "Apps".to_string(),
            sort_order: 1,
            view_type: ViewType::List,
            icon_size: 64,
        });
        config.data.items.push(Item {
            id: "item_x".to_string(),
            name: "Foo".to_string(),
            target: "C:\\foo.exe".to_string(),
            arguments: "--bar".to_string(),
            working_dir: "C:\\".to_string(),
            icon_path: "C:\\icon.ico".to_string(),
            cached_icon: String::new(),
            icon_index: 2,
            run_as_admin: true,
            run_count: 7,
            keywords: "foo bar".to_string(),
            remark: "test remark".to_string(),
            category_id: "cat_x".to_string(),
            sort_order: 3,
        });

        let path = std::env::temp_dir()
            .join("gorun_rs_full_test.json")
            .to_str()
            .unwrap()
            .to_string();
        let storage = Storage::new(path.clone());
        assert!(storage.save(&config));

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.config.window_x, 200);
        assert_eq!(loaded.config.window_y, 300);
        assert_eq!(loaded.config.window_width, 1024);
        assert_eq!(loaded.config.window_height, 768);
        assert_eq!(loaded.config.global_hotkey, "Ctrl+Alt+P");
        assert_eq!(
            loaded.extra_config.get("theme").map(|s| s.as_str()),
            Some("light")
        );
        assert_eq!(loaded.data.categories.len(), 1);
        assert_eq!(loaded.data.categories[0].view_type, ViewType::List);
        assert_eq!(loaded.data.categories[0].icon_size, 64);
        assert_eq!(loaded.data.items.len(), 1);
        let it = &loaded.data.items[0];
        assert_eq!(it.arguments, "--bar");
        assert_eq!(it.working_dir, "C:\\");
        assert_eq!(it.icon_path, "C:\\icon.ico");
        assert_eq!(it.icon_index, 2);
        assert!(it.run_as_admin);
        assert_eq!(it.run_count, 7);
        assert_eq!(it.keywords, "foo bar");
        assert_eq!(it.remark, "test remark");
        assert_eq!(it.category_id, "cat_x");

        let _ = std::fs::remove_file(&path);
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

    #[test]
    fn test_save_with_backup_rotates() {
        let path = std::env::temp_dir()
            .join(format!("gorun_rs_backup_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();

        let mut cfg_a = AppConfig::default();
        cfg_a.version = "A".into();
        let storage = Storage::new(path.clone());
        assert!(storage.save_with_backup(&cfg_a));

        let mut cfg_b = AppConfig::default();
        cfg_b.version = "B".into();
        assert!(storage.save_with_backup(&cfg_b));

        let bak1 = format!("{}.bak1", path);
        assert!(std::path::Path::new(&bak1).exists());
        let bak_cfg = Storage::new(bak1.clone()).load().unwrap();
        assert_eq!(bak_cfg.version, "A");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&bak1);
        let _ = std::fs::remove_file(&format!("{}.bak2", path));
        let _ = std::fs::remove_file(&format!("{}.bak3", path));
    }

    #[test]
    fn test_load_from_backup_when_main_corrupt() {
        let path = std::env::temp_dir()
            .join(format!("gorun_rs_corrupt_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();

        let mut cfg_a = AppConfig::default();
        cfg_a.version = "A".into();
        let storage = Storage::new(path.clone());
        assert!(storage.save_with_backup(&cfg_a));

        let mut cfg_b = AppConfig::default();
        cfg_b.version = "B".into();
        assert!(storage.save_with_backup(&cfg_b));

        std::fs::write(&path, "{ not valid json").unwrap();
        assert!(storage.load().is_none());

        let recovered = storage.load_from_backup().unwrap();
        assert_eq!(recovered.version, "A");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&format!("{}.bak1", path));
        let _ = std::fs::remove_file(&format!("{}.bak2", path));
        let _ = std::fs::remove_file(&format!("{}.bak3", path));
    }

    #[test]
    fn test_atomic_write_no_tmp_left() {
        let path = std::env::temp_dir()
            .join(format!("gorun_rs_tmp_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();

        let config = AppConfig::default();
        let storage = Storage::new(path.clone());
        assert!(storage.save(&config));

        let tmp = format!("{}.tmp.{}", path, std::process::id());
        assert!(!std::path::Path::new(&tmp).exists());

        let _ = std::fs::remove_file(&path);
    }
}
