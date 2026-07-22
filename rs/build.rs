fn main() {
    slint_build::compile("ui/main_window.slint").unwrap();
    slint_build::compile("ui/edit_dialog.slint").unwrap();
}
