mod commands;

use tauri_plugin_sql::{Migration, MigrationKind};

fn db_migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "init schema",
        sql: include_str!("../migrations/001_init.sql"),
        kind: MigrationKind::Up,
    }]
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:winplayer.db", db_migrations())
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::http::http_fetch,
            commands::plugins::plugin_list,
            commands::plugins::plugin_read_code,
            commands::plugins::plugin_install_from_path,
            commands::plugins::plugin_install_from_url,
            commands::plugins::plugin_remove,
            commands::plugins::plugin_storage_get,
            commands::plugins::plugin_storage_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
