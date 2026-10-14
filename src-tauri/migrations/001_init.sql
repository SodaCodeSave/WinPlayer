-- WinPlayer 初始数据库结构（SQLite）
-- 注意：此文件通过 tauri-plugin-sql 的迁移机制在首次启动时执行

create table if not exists settings (
  key   text primary key,
  value text not null
);

-- 我喜欢的音乐（track 列保存 UnifiedTrack JSON，跨插件源统一结构）
create table if not exists liked_songs (
  id        integer primary key autoincrement,
  source_id text    not null,
  track_id  text    not null,
  track     text    not null,
  liked_at  integer not null,
  unique (source_id, track_id)
);

-- 本地自定义歌单
create table if not exists playlists (
  id          integer primary key autoincrement,
  name        text    not null,
  description text    not null default '',
  cover       text    not null default '',
  created_at  integer not null,
  updated_at  integer not null
);

create table if not exists playlist_tracks (
  id          integer primary key autoincrement,
  playlist_id integer not null,
  track       text    not null,
  position    integer not null,
  added_at    integer not null,
  foreign key (playlist_id) references playlists (id) on delete cascade
);

create index if not exists idx_playlist_tracks_playlist on playlist_tracks (playlist_id, position);

-- 插件启用状态与参数配置（settings 为 JSON 对象）
create table if not exists plugin_state (
  plugin_id text primary key,
  enabled   integer not null default 1,
  settings  text    not null default '{}'
);
