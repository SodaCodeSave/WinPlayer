import { createRouter, createWebHashHistory } from "vue-router";

/**
 * 使用 hash 路由：Tauri 自定义协议下避免刷新时路径 404
 */
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "recommend",
      component: () => import("@/views/RecommendView.vue"),
      meta: { title: "推荐" },
    },
    {
      path: "/search",
      name: "search",
      component: () => import("@/views/SearchView.vue"),
      meta: { title: "搜索" },
    },
    {
      path: "/liked",
      name: "liked",
      component: () => import("@/views/LikedView.vue"),
      meta: { title: "我喜欢的音乐" },
    },
    {
      path: "/playlist/:id",
      name: "playlist",
      component: () => import("@/views/PlaylistView.vue"),
      meta: { title: "歌单" },
    },
    {
      path: "/plugins",
      name: "plugins",
      component: () => import("@/views/PluginsView.vue"),
      meta: { title: "插件管理" },
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("@/views/SettingsView.vue"),
      meta: { title: "设置" },
    },
    {
      path: "/:pathMatch(.*)*",
      redirect: "/",
    },
  ],
});

router.afterEach((to) => {
  document.title = to.meta.title ? `${to.meta.title} - WinPlayer` : "WinPlayer";
});

export default router;
