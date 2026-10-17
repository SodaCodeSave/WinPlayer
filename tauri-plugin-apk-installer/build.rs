/// 本插件不向前端暴露任何命令（安装动作完全由 Rust 侧调用），
/// 因此命令列表为空；build 脚本只负责声明 Android 工程目录，
/// 让 Tauri CLI 把它加入生成的 Android 工程。
fn main() {
    tauri_plugin::Builder::new(&[])
        .android_path("android")
        .build();
}
