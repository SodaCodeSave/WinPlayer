package app.tauri.apkinstaller

import androidx.core.content.FileProvider

/**
 * 独立的 FileProvider 子类。
 *
 * 清单合并是按 `android:name` 归并组件的：Tauri 的 Android 模板已在应用清单中声明了
 * `androidx.core.content.FileProvider`（authority 为 `${applicationId}.fileprovider`，
 * meta-data 指向 `@xml/file_paths`）。若本插件再用同一个类名声明一个 provider，
 * 合并时会在 `android:authorities` 与 `meta-data#android.support.FILE_PROVIDER_PATHS`
 * 上产生取值冲突，导致 `Manifest merger failed` 构建失败。
 *
 * 换成本插件专属的子类名后，两个 provider 会被视为不同组件并存；
 * `FileProvider.getUriForFile` 按 authority 解析，子类同样可用。
 */
class ApkFileProvider : FileProvider()
