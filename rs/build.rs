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
}
