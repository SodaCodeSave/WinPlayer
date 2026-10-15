/** 「添加到歌单」选择器（全局共享对话框） */

import { reactive } from "vue";
import type { UnifiedTrack } from "@/types/plugin";

interface PlaylistPickerState {
  visible: boolean;
  track: UnifiedTrack | null;
}

export const playlistPickerState = reactive<PlaylistPickerState>({
  visible: false,
  track: null,
});

export function openPlaylistPicker(track: UnifiedTrack): void {
  playlistPickerState.track = track;
  playlistPickerState.visible = true;
}

export function closePlaylistPicker(): void {
  playlistPickerState.visible = false;
  playlistPickerState.track = null;
}
