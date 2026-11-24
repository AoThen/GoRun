fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    slint_build::compile("ui/main_window.slint").unwrap();

    slint_build::compile("ui/edit_dialog.slint").unwrap();

    println!(
        "cargo:rustc-env=SLINT_INCLUDE_GENERATED={}/main_window.rs",
        out_dir
    );
    println!(
        "cargo:rustc-env=SLINT_INCLUDE_GENERATED_EDIT_DIALOG={}/edit_dialog.rs",
        out_dir
    );

    // Windows 产物嵌入应用图标（资源 ID 1）：exe 文件图标、任务栏 fallback、托盘加载共用
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let icon = std::path::Path::new(&manifest_dir).join("../resources/icons/app.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon(&icon.to_string_lossy());
        res.compile().unwrap();
    }
}
