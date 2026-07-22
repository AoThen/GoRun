# GoRun Rust 窗口闪退问题修复计划

## 问题诊断

程序启动后日志显示：
1. Config file not found → 创建了默认分类（正常回退）
2. Language file not found → `lang/zh-CN.json` 找不到（正常回退）
3. 日志在此中断，没有 `"MainWindow created successfully"`

**结论**：程序在 `main.rs:50` 的 `MainWindow::new().unwrap()` 处 panic，但由于 `Cargo.toml` 设置了 `windows_subsystem = "windows"`，panic 信息无处显示，窗口一闪而过。

## 修复方案

### 修改 1：添加 panic hook（`logger.rs`）

在 `init()` 后调用 `set_panic_hook()`，将 panic 信息写入日志文件。

**修改文件**：`rs/src/logger.rs`

```rust
pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "unknown panic"
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "?".to_string());
        log::error!("PANIC at {}: {}", location, msg);
        log::logger().flush();
    }));
}
```

### 修改 2：修复路径查找（`localization.rs`）

语言文件支持多路径查找：优先 exe 同级 `lang/`，回退到 CWD。

**修改文件**：`rs/src/localization.rs` 的 `lang_file_path` 方法

```rust
fn lang_file_path(code: &str) -> PathBuf {
    let relative = format!("{}/{}.json", LANG_DIR, code);

    // 优先：exe 同级目录
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let exe_lang = exe_dir.join(&relative);
            if exe_lang.exists() {
                return exe_lang;
            }
        }
    }

    // 回退：当前工作目录
    PathBuf::from(relative)
}
```

### 修改 3：配置便携模式（`config.rs`）

exe 同级 `config.json` 优先，不存在时使用 `%APPDATA%`。

**修改文件**：`rs/src/config.rs`

```rust
pub fn config_path() -> PathBuf {
    // 便携模式：exe 同级 config.json 优先
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
    log::debug!("Config path: {:?}", path)
}
```

### 修改 4：注册 panic hook（`main.rs`）

在 `logger::init()` 后立即调用 `logger::set_panic_hook()`。

**修改文件**：`rs/src/main.rs` 第 22 行后添加

```rust
fn main() {
    logger::init().ok();
    logger::set_panic_hook();  // ← 新增
    log::info!("GoRun starting...");
```

## 涉及文件清单

| 文件 | 修改类型 |
|------|----------|
| `rs/src/logger.rs` | 新增 `set_panic_hook()` 函数 |
| `rs/src/localization.rs` | 重写 `lang_file_path()` 支持多路径 |
| `rs/src/config.rs` | 重写 `config_path()` 支持便携模式 |
| `rs/src/main.rs` | 添加 `set_panic_hook()` 调用 |

## 验证计划

1. `cargo build --release` 编译通过
2. 将 release 目录的 exe 复制到独立空目录运行
3. 检查 `gorun_debug.log` 确认无 panic
4. 确认 `lang/zh-CN.json` 从 exe 目录正确加载
5. 确认 exe 目录生成 `config.json`（便携模式）

## 预期效果

- 即使再次崩溃，`gorun_debug.log` 会记录完整 panic 信息
- 语言文件在 exe 同级的 `lang/` 目录即可找到
- 配置文件优先存储在 exe 同级目录，实现绿色便携
