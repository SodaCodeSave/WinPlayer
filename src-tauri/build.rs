fn main() {
    // Android 上动态链接 C++ 运行时：cpal 的 Oboe 宿主是 C++ 库，
    // 新版 NDK 不再把 libc++ 静态链入，缺 __cxa_pure_virtual 等符号
    // 会导致 dlopen 失败（应用启动即闪退）。libc++_shared.so 由
    // CI 在 gen/android/app/src/main/jniLibs 提供打包。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-lib=dylib=c++_shared");
    }
    tauri_build::build()
}
