/** 播放器状态与控制（队列 / 模式 / 歌词），底层为 HTML5 Audio 引擎 */

import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { getAudioEngine } from "@/services/audio";
import { findLyricIndex, mergeTranslation, parseLrc } from "@/services/lrc";
import { getSetting, setSetting } from "@/services/db";
import { toast } from "@/services/toast";
import { usePluginsStore } from "./plugins";
import { createQueueUid, type LyricLine, type PlayMode, type QueueItem } from "@/types/player";
import type { UnifiedTrack } from "@/types/plugin";

const MODE_ORDER: PlayMode[] = ["sequential", "loop-all", "loop-one", "shuffle"];

export const usePlayerStore = defineStore("player", () => {
  const engine = getAudioEngine();
  const plugins = usePluginsStore();

  const queue = ref<QueueItem[]>([]);
  const currentIndex = ref(-1);
  const mode = ref<PlayMode>("loop-all");
  const playing = ref(false);
  const loading = ref(false);
  const currentTime = ref(0);
  const duration = ref(0);
  const volume = ref(0.8);
  const muted = ref(false);

  /** 当前已加载音频的曲目标识（用于判断是否需要解析直链） */
  let loadedTrackKey: string | null = null;

  // 歌词
  const lyrics = ref<LyricLine[]>([]);
  const lyricsLoading = ref(false);
  const lyricIndex = computed(() => findLyricIndex(lyrics.value, currentTime.value * 1000));

  // UI 状态（队列/歌词面板）
  const showQueue = ref(false);
  const showLyrics = ref(false);

  const currentTrack = computed<UnifiedTrack | null>(() =>
    currentIndex.value >= 0 && currentIndex.value < queue.value.length
      ? queue.value[currentIndex.value].track
      : null,
  );

  /* ------------------------------ 引擎事件 ------------------------------ */

  engine.on("timeupdate", (time) => {
    currentTime.value = time;
  });
  engine.on("durationchange", (value) => {
    if (value > 0) duration.value = value;
  });
  engine.on("playing", () => {
    playing.value = true;
    loading.value = false;
  });
  engine.on("paused", () => {
    playing.value = false;
  });
  engine.on("waiting", () => {
    loading.value = true;
  });
  engine.on("ended", () => {
    handleEnded();
  });
  engine.on("error", (message) => {
    loading.value = false;
    playing.value = false;
    toast(message, "error");
  });

  /* ------------------------------ 播放控制 ------------------------------ */

  function trackUid(track: UnifiedTrack): string {
    return `${track.sourceId ?? ""}:${track.id}`;
  }

  /** 播放队列中指定位置的曲目 */
  async function playAt(index: number): Promise<void> {
    if (index < 0 || index >= queue.value.length) return;
    currentIndex.value = index;
    const track = queue.value[index].track;
    loading.value = true;
    try {
      const result = await plugins.getSongUrl(track);
      loadedTrackKey = trackUid(track);
      engine.load(result.url);
      engine.volume = volume.value;
      engine.muted = muted.value;
      await engine.play();
      void fetchLyrics(track);
      persistQueue();
    } catch (error) {
      loading.value = false;
      playing.value = false;
      toast(`无法播放「${track.name}」: ${error}`, "error");
    }
  }

  /** 用一组曲目替换队列并从指定索引播放 */
  async function playTracks(tracks: UnifiedTrack[], startIndex = 0): Promise<void> {
    if (!tracks.length) return;
    queue.value = tracks.map((track) => ({ uid: createQueueUid(), track }));
    await playAt(Math.min(Math.max(0, startIndex), tracks.length - 1));
  }

  /** 将曲目追加到队列尾部（不在队列时） */
  function addToQueue(tracks: UnifiedTrack[]): void {
    const existing = new Set(queue.value.map((item) => trackUid(item.track)));
    let added = 0;
    for (const track of tracks) {
      const key = trackUid(track);
      if (existing.has(key)) continue;
      existing.add(key);
      queue.value.push({ uid: createQueueUid(), track });
      added += 1;
    }
    if (added) {
      toast(`已添加 ${added} 首到播放队列`, "success");
      persistQueue();
    } else {
      toast("歌曲已在播放队列中", "info");
    }
  }

  /** 播放单曲（并以给定上下文重建队列） */
  async function playTrack(track: UnifiedTrack, context?: UnifiedTrack[]): Promise<void> {
    if (context?.length) {
      const index = context.findIndex((t) => trackUid(t) === trackUid(track));
      await playTracks(context, index >= 0 ? index : 0);
      return;
    }
    if (queue.value.length && currentIndex.value >= 0) {
      // 插入到当前曲目之后
      const insertAt = currentIndex.value + 1;
      queue.value.splice(insertAt, 0, { uid: createQueueUid(), track });
      await playAt(insertAt);
    } else {
      await playTracks([track], 0);
    }
  }

  async function togglePlay(): Promise<void> {
    if (currentIndex.value < 0 || !queue.value.length) {
      // 恢复了队列但尚未加载音频：直接加载当前曲目
      if (queue.value.length) await playAt(Math.max(0, currentIndex.value));
      return;
    }
    const track = queue.value[currentIndex.value].track;
    if (loadedTrackKey !== trackUid(track)) {
      // 队列恢复场景：首次播放需解析直链
      await playAt(currentIndex.value);
      return;
    }
    if (engine.paused) {
      try {
        await engine.play();
      } catch (error) {
        toast(`播放失败: ${error}`, "error");
      }
    } else {
      engine.pause();
    }
  }

  function handleEnded(): void {
    if (mode.value === "loop-one") {
      engine.seek(0);
      void engine.play();
      return;
    }
    if (mode.value === "sequential" && currentIndex.value >= queue.value.length - 1) {
      playing.value = false;
      return;
    }
    void next(true);
  }

  async function next(auto = false): Promise<void> {
    if (!queue.value.length) return;
    if (mode.value === "shuffle") {
      if (queue.value.length === 1) {
        await playAt(0);
        return;
      }
      let index = currentIndex.value;
      while (index === currentIndex.value) {
        index = Math.floor(Math.random() * queue.value.length);
      }
      await playAt(index);
      return;
    }
    let index = currentIndex.value + 1;
    if (index >= queue.value.length) {
      if (mode.value === "sequential" && auto) {
        playing.value = false;
        return;
      }
      index = 0;
    }
    await playAt(index);
  }

  async function prev(): Promise<void> {
    if (!queue.value.length) return;
    // 播放超过 3 秒时，先回到开头
    if (currentTime.value > 3) {
      engine.seek(0);
      currentTime.value = 0;
      return;
    }
    let index = currentIndex.value - 1;
    if (index < 0) index = queue.value.length - 1;
    await playAt(index);
  }

  function seek(seconds: number): void {
    engine.seek(seconds);
    currentTime.value = seconds;
  }

  async function setVolume(value: number): Promise<void> {
    volume.value = Math.min(1, Math.max(0, value));
    engine.volume = volume.value;
    if (muted.value && volume.value > 0) {
      muted.value = false;
      engine.muted = false;
    }
    await setSetting("player.volume", volume.value);
  }

  async function toggleMute(): Promise<void> {
    muted.value = !muted.value;
    engine.muted = muted.value;
    await setSetting("player.muted", muted.value);
  }

  function cycleMode(): void {
    const index = MODE_ORDER.indexOf(mode.value);
    mode.value = MODE_ORDER[(index + 1) % MODE_ORDER.length];
    void setSetting("player.mode", mode.value);
  }

  /* ------------------------------ 队列管理 ------------------------------ */

  function removeFromQueue(uid: string): void {
    const index = queue.value.findIndex((item) => item.uid === uid);
    if (index < 0) return;
    queue.value.splice(index, 1);
    if (index < currentIndex.value) {
      currentIndex.value -= 1;
    } else if (index === currentIndex.value) {
      // 移除的是正在播放的曲目
      engine.stop();
      playing.value = false;
      loadedTrackKey = null;
      if (queue.value.length) {
        void playAt(Math.min(index, queue.value.length - 1));
      } else {
        currentIndex.value = -1;
      }
    }
    persistQueue();
  }

  function clearQueue(): void {
    queue.value = [];
    currentIndex.value = -1;
    loadedTrackKey = null;
    engine.stop();
    playing.value = false;
    lyrics.value = [];
    persistQueue();
  }

  function moveQueueItem(from: number, to: number): void {
    if (from === to || from < 0 || to < 0 || from >= queue.value.length || to >= queue.value.length) {
      return;
    }
    const currentUid = currentTrack.value ? queue.value[currentIndex.value]?.uid : null;
    const [item] = queue.value.splice(from, 1);
    queue.value.splice(to, 0, item);
    if (currentUid) {
      currentIndex.value = queue.value.findIndex((entry) => entry.uid === currentUid);
    }
    persistQueue();
  }

  /* ------------------------------ 歌词 ------------------------------ */

  async function fetchLyrics(track: UnifiedTrack): Promise<void> {
    lyricsLoading.value = true;
    lyrics.value = [];
    try {
      const result = await plugins.getLyrics(track);
      if (result?.lrc) {
        const lines = parseLrc(result.lrc);
        lyrics.value = mergeTranslation(lines, result.translation);
      }
    } finally {
      lyricsLoading.value = false;
    }
  }

  /* ------------------------------ 持久化 ------------------------------ */

  let persistTimer: number | undefined;

  function persistQueue(): void {
    window.clearTimeout(persistTimer);
    persistTimer = window.setTimeout(() => {
      const snapshot = {
        tracks: queue.value.map((item) => item.track),
        index: currentIndex.value,
        mode: mode.value,
      };
      void setSetting("player.queue", snapshot);
    }, 600);
  }

  watch(mode, persistQueue);

  /** 启动时恢复（不自动播放） */
  async function restore(): Promise<void> {
    volume.value = await getSetting("player.volume", 0.8);
    muted.value = await getSetting("player.muted", false);
    const savedMode = await getSetting<PlayMode>("player.mode", "loop-all");
    if (MODE_ORDER.includes(savedMode)) mode.value = savedMode;
    engine.volume = volume.value;
    engine.muted = muted.value;

    const snapshot = await getSetting<{
      tracks?: UnifiedTrack[];
      index?: number;
    } | null>("player.queue", null);
    if (snapshot?.tracks?.length) {
      queue.value = snapshot.tracks.map((track) => ({ uid: createQueueUid(), track }));
      currentIndex.value = Math.min(Math.max(0, snapshot.index ?? 0), queue.value.length - 1);
      duration.value = queue.value[currentIndex.value].track.duration
        ? queue.value[currentIndex.value].track.duration! / 1000
        : 0;
    }
  }

  return {
    queue,
    currentIndex,
    mode,
    playing,
    loading,
    currentTime,
    duration,
    volume,
    muted,
    lyrics,
    lyricsLoading,
    lyricIndex,
    showQueue,
    showLyrics,
    currentTrack,
    playAt,
    playTracks,
    playTrack,
    addToQueue,
    togglePlay,
    next,
    prev,
    seek,
    setVolume,
    toggleMute,
    cycleMode,
    removeFromQueue,
    clearQueue,
    moveQueueItem,
    restore,
    persistQueue,
  };
});
