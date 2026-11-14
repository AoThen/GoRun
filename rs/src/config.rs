use std::path::{Path, PathBuf};

/// 便携优先的基准目录解析：exe 目录可写 → 便携；否则回退系统数据目录。
/// 核心逻辑抽为纯函数 `base_dir_from`，便于单测注入探测函数。
pub fn base_dir_from(exe_dir: &Path, probe: &dyn Fn(&Path) -> bool) -> PathBuf {
    if probe(exe_dir) {
        log::debug!("Using portable base dir: {:?}", exe_dir);
        exe_dir.to_path_buf()
    } else {
        let fallback = app_data_fallback();
        log::warn!("Exe dir not writable (or missing), fallback to AppData: {:?}", fallback);
        fallback
    }
}

/// 默认探测：在目录内尝试创建/删除临时文件。
pub fn default_writable_probe(dir: &Path) -> bool {
    let probe_file = dir.join(format!(".gorun_write_probe_{}", std::process::id()));
    match std::fs::File::create(&probe_file) {
        Ok(_) => {
            if let Err(e) = std::fs::remove_file(&probe_file) {
                log::warn!("Failed to remove writability probe file {:?}: {}", probe_file, e);
            }
            true
        }
        Err(_) => false,
    }
}

fn app_data_fallback() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join("GoRun"))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 解析并缓存基准目录（进程级）。
pub fn base_dir() -> PathBuf {
    static BASE_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BASE_DIR
        .get_or_init(|| {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|p| p.to_path_buf()));
            match exe_dir {
                Some(dir) => base_dir_from(&dir, &default_writable_probe),
                None => app_data_fallback(),
            }
        })
        .clone()
}

pub fn config_path() -> PathBuf {
    base_dir().join("config.json")
}

pub fn icons_dir() -> PathBuf {
    base_dir().join("icons")
}

/// 迁移旧版本（AppData 落盘）数据到当前基准目录：
/// 复制 config.json、.bak1-3 与 icons 目录；只复制不删源；单文件失败仅告警。
/// 旧目录与当前基准目录相同（如回退模式）时直接跳过。
pub fn migrate_legacy() {
    migrate_legacy_from(&app_data_fallback(), &base_dir());
}

/// 迁移主体（独立纯函数，便于单测真实驱动）。
pub fn migrate_legacy_from(old: &Path, current: &Path) {
    if old == current {
        log::debug!("Legacy migration skipped: base dir == AppData fallback");
        return;
    }
    if !old.join("config.json").exists() {
        log::debug!("Legacy migration skipped: no old config in {:?}", old);
        return;
    }
    let target = current.join("config.json");
    if target.exists() {
        log::info!("Legacy migration skipped: portable config already exists");
        return;
    }

    log::info!("Migrating legacy data from {:?} to {:?}", old, current);
    let mut any_fail = false;

    for name in ["config.json", "config.json.bak1", "config.json.bak2", "config.json.bak3"] {
        let src = old.join(name);
        if !src.exists() {
            continue;
        }
        let dst = current.join(name);
        if let Err(e) = std::fs::copy(&src, &dst) {
            any_fail = true;
            log::error!("Failed to copy {}: {}", name, e);
        }
    }

    let old_icons = old.join("icons");
    if old_icons.exists() {
        let new_icons = current.join("icons");
        if let Err(e) = std::fs::create_dir_all(&new_icons) {
            any_fail = true;
            log::error!("Failed to create icons dir {:?}: {}", new_icons, e);
        } else if let Ok(entries) = std::fs::read_dir(&old_icons) {
            for entry in entries.flatten() {
                let src = entry.path();
                let dst = new_icons.join(entry.file_name());
                if src.is_file() {
                    if let Err(e) = std::fs::copy(&src, &dst) {
                        any_fail = true;
                        log::error!("Failed to copy icon {:?}: {}", src, e);
                    }
                }
            }
        }
    }

    if any_fail {
        log::warn!("Legacy migration completed with partial failures");
    } else {
        log::info!("Legacy migration completed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_dir_writable_uses_exe() {
        let dir = std::env::temp_dir().join("gorun_base_test_w");
        let _ = std::fs::create_dir_all(&dir);
        let out = base_dir_from(&dir, &|_| true);
        assert_eq!(out, dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_base_dir_readonly_falls_back_to_appdata() {
        let dir = std::env::temp_dir().join("gorun_base_test_ro");
        let _ = std::fs::create_dir_all(&dir);
        let out = base_dir_from(&dir, &|_| false);
        assert_ne!(out, dir);
        assert!(out.ends_with("GoRun"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_config_path_under_base_dir() {
        let dir = std::env::temp_dir().join("gorun_base_test_cfg");
        let _ = std::fs::create_dir_all(&dir);
        let path = base_dir_from(&dir, &|_| true).join("config.json");
        assert_eq!(path.parent().unwrap(), dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_migrate_copies_and_keeps_source() {
        let old = std::env::temp_dir().join("gorun_migrate_old");
        let cur = std::env::temp_dir().join("gorun_migrate_cur");
        let _ = std::fs::create_dir_all(old.join("icons"));
        let _ = std::fs::create_dir_all(&cur);
        std::fs::write(old.join("config.json"), b"{\"version\":\"1.0.0\"}").unwrap();
        std::fs::write(old.join("config.json.bak1"), b"bak1").unwrap();
        std::fs::write(old.join("icons").join("abc.png"), b"png").unwrap();

        migrate_legacy_from(&old, &cur);

        // 复制且源保留
        assert!(cur.join("config.json").exists());
        assert!(cur.join("config.json.bak1").exists());
        assert!(cur.join("icons").join("abc.png").exists());
        assert!(old.join("config.json").exists());
        assert!(old.join("icons").join("abc.png").exists());

        let _ = std::fs::remove_dir_all(&old);
        let _ = std::fs::remove_dir_all(&cur);
    }

    #[test]
    fn test_migrate_skips_when_current_exists() {
        let old = std::env::temp_dir().join("gorun_migrate_old2");
        let cur = std::env::temp_dir().join("gorun_migrate_cur2");
        let _ = std::fs::create_dir_all(&old);
        let _ = std::fs::create_dir_all(&cur);
        std::fs::write(old.join("config.json"), b"old").unwrap();
        std::fs::write(cur.join("config.json"), b"current").unwrap();

        migrate_legacy_from(&old, &cur);

        // 已存在便携配置时不覆盖
        assert_eq!(std::fs::read(cur.join("config.json")).unwrap(), b"current");

        let _ = std::fs::remove_dir_all(&old);
        let _ = std::fs::remove_dir_all(&cur);
    }

    #[test]
    fn test_migrate_skips_when_no_old_config() {
        let old = std::env::temp_dir().join("gorun_migrate_old_empty");
        let cur = std::env::temp_dir().join("gorun_migrate_cur_empty");
        let _ = std::fs::create_dir_all(&old);
        let _ = std::fs::create_dir_all(&cur);
        // 旧目录无 config.json → 直接跳过，不创建任何文件
        migrate_legacy_from(&old, &cur);
        assert!(!cur.join("config.json").exists(), "无旧配置时不应迁移");
        let _ = std::fs::remove_dir_all(&old);
        let _ = std::fs::remove_dir_all(&cur);
    }

    #[test]
    fn test_migrate_copies_all_backups_and_multiple_icons() {
        let old = std::env::temp_dir().join("gorun_migrate_old_full");
        let cur = std::env::temp_dir().join("gorun_migrate_cur_full");
        let _ = std::fs::create_dir_all(old.join("icons"));
        let _ = std::fs::create_dir_all(&cur);
        std::fs::write(old.join("config.json"), b"main").unwrap();
        std::fs::write(old.join("config.json.bak1"), b"b1").unwrap();
        std::fs::write(old.join("config.json.bak2"), b"b2").unwrap();
        std::fs::write(old.join("config.json.bak3"), b"b3").unwrap();
        std::fs::write(old.join("icons").join("a.png"), b"a").unwrap();
        std::fs::write(old.join("icons").join("b.ico"), b"b").unwrap();

        migrate_legacy_from(&old, &cur);

        for f in ["config.json", "config.json.bak1", "config.json.bak2", "config.json.bak3"] {
            assert!(cur.join(f).exists(), "{} 应被复制", f);
        }
        assert!(cur.join("icons").join("a.png").exists());
        assert!(cur.join("icons").join("b.ico").exists());
        let _ = std::fs::remove_dir_all(&old);
        let _ = std::fs::remove_dir_all(&cur);
    }

    #[test]
    fn test_migrate_skips_same_dir() {
        let dir = std::env::temp_dir().join("gorun_migrate_same");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("config.json"), b"x").unwrap();
        migrate_legacy_from(&dir, &dir);
        assert_eq!(std::fs::read(dir.join("config.json")).unwrap(), b"x");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
