fn main() {
    slint_build::compile("ui/app.slint").expect("failed to compile ui/app.slint");

    // Embed the icon in the .exe so Explorer, shortcuts and the taskbar show it.
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("icons/icon.ico")
        .compile()
        .expect("failed to embed the Windows icon");
}
