package app.tauri.apkinstaller

import android.app.Activity
import android.content.Intent
import android.util.Log
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import java.io.File

private const val TAG = "ApkInstaller"
private const val AUTHORITY = "app.tauri.apkinstaller.fileprovider"
private const val APK_MIME = "application/vnd.android.package-archive"

@InvokeArg
class InstallApkArgs {
    lateinit var path: String
}

@TauriPlugin
class ApkInstallerPlugin(private val activity: Activity) : Plugin(activity) {

    /**
     * 调起系统软件包安装器安装指定路径的 APK。
     *
     * FileProvider 生成的 content:// URI 必须显式授予读权限，否则安装器
     * 无法读取安装包；ACTION_VIEW 需要 NEW_TASK，因为是从 Activity 之外调起。
     */
    @Command
    fun installApk(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(InstallApkArgs::class.java)
            val apk = File(args.path)
            if (!apk.isFile) {
                invoke.reject("安装包不存在：${args.path}")
                return
            }

            val uri = FileProvider.getUriForFile(activity, AUTHORITY, apk)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, APK_MIME)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            activity.applicationContext?.startActivity(intent)
            Log.d(TAG, "install intent started for ${args.path}")
            invoke.resolve()
        } catch (ex: Exception) {
            Log.e(TAG, "install apk failed", ex)
            invoke.reject(ex.message ?: "调起安装器失败")
        }
    }
}
