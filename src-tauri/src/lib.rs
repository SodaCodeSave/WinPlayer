mod commands;
mod player;
mod smtc;

use tauri_plugin_sql::{Migration, MigrationKind};

fn db_migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "init schema",
        sql: include_str!("../migrations/001_init.sql"),
        kind: MigrationKind::Up,
    }]
}

// 移动端通过动态库入口启动，需要导出 tauri 的 mobile entry point 符号
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = commands::cache::register(
        tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(
                tauri_plugin_sql::Builder::default()
                    .add_migrations("sqlite:winplayer.db", db_migrations())
                    .build(),
            )
            // invoke_handler 只能调用一次（再次调用会整体替换命令表），
            // 所有命令必须集中在此列出
            .invoke_handler(tauri::generate_handler![
                commands::cache::cache_stats,
                commands::cache::cache_clear,
                commands::cache::image_cache_prefetch,
                commands::http::http_fetch,
                commands::plugins::plugin_list,
                commands::plugins::plugin_read_code,
                commands::plugins::plugin_install_from_path,
                commands::plugins::plugin_install_from_url,
                commands::plugins::plugin_remove,
                commands::plugins::plugin_storage_get,
                commands::plugins::plugin_storage_set,
                commands::smtc::smtc_sync,
                player::player_load,
                player::player_play,
                player::player_pause,
                player::player_stop,
                player::player_seek,
                player::player_set_volume,
                player::player_set_eq,
            ])
            .setup(|app| {
                // 主窗口此时已创建，可在其上挂接系统媒体传输控件（SMTC）
                smtc::init(app.handle());
                // 原生音频播放引擎（命令 + 事件见 src-tauri/src/player/）
                player::init(app);
                Ok(())
            }),
    );
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}