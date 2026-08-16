fn main() {
    println!("cargo:rerun-if-changed=../app-icon.ico");

    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("../app-icon.ico")
        .compile()
        .expect("无法写入 Windows 应用图标");
}
