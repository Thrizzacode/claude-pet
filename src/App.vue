<template>
  <div
    class="pet-container"
    data-tauri-drag-region
    @contextmenu.prevent="showContextMenu"
  >
    <!-- 更新通知 banner -->
    <div v-if="updateInfo" class="update-banner" @click.stop>
      <span>新版本 {{ updateInfo.version }} 可更新</span>
      <div class="update-actions">
        <button class="update-btn" @click="doInstallUpdate" :disabled="updating">
          {{ updating ? "安裝中..." : "立即更新" }}
        </button>
        <button class="update-dismiss" @click="updateInfo = null">✕</button>
      </div>
    </div>
    <!-- 已是最新版本 toast -->
    <div v-if="upToDateToast" class="update-toast" @click.stop>已是最新版本</div>
    <div class="pet-wrapper">
      <img v-if="petState === 'pending'" class="pending-badge" :src="alertImg" />
      <img class="pet-character" :class="petState" :src="petAppearance" />
    </div>

    <!-- 右鍵選單遮罩 -->
    <div
      v-if="contextMenu.visible"
      class="context-menu-backdrop"
      @click="hideContextMenu"
      @contextmenu.prevent="hideContextMenu"
    ></div>

    <!-- 右鍵選單 -->
    <div
      v-if="contextMenu.visible"
      ref="contextMenuEl"
      class="context-menu"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      @click.stop
    >
      <div class="context-menu-item" @click="selectCharacter('zeztz')">
        <span>Zeztz</span>
        <span
          class="context-menu-check"
          :style="{
            visibility: selectedCharacter === 'zeztz' ? 'visible' : 'hidden',
          }"
          >✓</span
        >
      </div>
      <div class="context-menu-item" @click="selectCharacter('border-collie')">
        <span>Border Collie</span>
        <span
          class="context-menu-check"
          :style="{
            visibility:
              selectedCharacter === 'border-collie' ? 'visible' : 'hidden',
          }"
          >✓</span
        >
      </div>
      <div class="context-menu-item" @click="selectCharacter('rider1')">
        <span>Rider1</span>
        <span
          class="context-menu-check"
          :style="{
            visibility: selectedCharacter === 'rider1' ? 'visible' : 'hidden',
          }"
          >✓</span
        >
      </div>
      <div class="context-menu-item" @click="selectCharacter('v3')">
        <span>V3</span>
        <span
          class="context-menu-check"
          :style="{
            visibility: selectedCharacter === 'v3' ? 'visible' : 'hidden',
          }"
          >✓</span
        >
      </div>
      <div class="context-menu-divider"></div>
      <div class="context-menu-item" @click="openSettings">設定</div>
      <div class="context-menu-divider"></div>
      <div class="context-menu-version">v{{ appVersion }}</div>
    </div>

    <!-- 設定 Dialog -->
    <div
      v-if="settingsOpen"
      class="dialog-overlay"
      @click.self="settingsOpen = false"
    >
      <div class="dialog">
        <div class="dialog-title">設定</div>
        <label class="toggle-row">
          <span>開機自動啟動</span>
          <input
            type="checkbox"
            v-model="autostartEnabled"
            @change="toggleAutostart"
          />
        </label>
        <div class="hooks-section">
          <div class="hooks-label">安裝 Claude Hooks</div>
          <div class="hooks-buttons">
            <button class="hooks-btn" @click="setupHooks('global')" :disabled="!!hooksStatus">全域</button>
            <button class="hooks-btn" @click="setupHooks('project')" :disabled="!!hooksStatus">專案</button>
          </div>
          <span v-if="hooksStatus === 'ok'" class="hooks-ok">已安裝 ✓</span>
          <span v-if="hooksStatus === 'error'" class="hooks-error">安裝失敗</span>
          <span v-if="hooksStatus === 'loading'" class="hooks-loading">安裝中...</span>
          <span v-if="hooksStatus === 'cancelled'" class="hooks-hint">已取消</span>
        </div>
        <button class="dialog-close" @click="settingsOpen = false">關閉</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import alertImg from "./assets/imgs/alert.png";
import zeztzIdleImg from "./assets/imgs/Zeztz/zeztz-idle.png";
import zeztzNotifiedImg from "./assets/imgs/Zeztz/zeztz-notified.png";
import dogIdleImg from "./assets/imgs/Border Collie/dog-idle.png";
import dogNotifiedImg from "./assets/imgs/Border Collie/dog-notified.png";
import rider1IdleImg from "./assets/imgs/Rider1/rider1-idle.png";
import rider1NotifiedImg from "./assets/imgs/Rider1/rider1-notified.png";
import v3IdleImg from "./assets/imgs/V3/v3-idle.png";
import v3NotifiedImg from "./assets/imgs/V3/v3-notified.png";

type Character = "zeztz" | "border-collie" | "rider1" | "v3";

interface UpdateInfo {
  version: string;
  currentVersion: string;
  body?: string;
}

const updateInfo = ref<UpdateInfo | null>(null);
const updating = ref(false);
const upToDateToast = ref(false);
let upToDateTimer: ReturnType<typeof setTimeout> | null = null;

function showUpToDateToast() {
  if (upToDateTimer) clearTimeout(upToDateTimer);
  upToDateToast.value = true;
  upToDateTimer = setTimeout(() => { upToDateToast.value = false; }, 3000);
}

const petState = ref<"idle" | "notified" | "pending">("idle");
let notifiedTimer: ReturnType<typeof setTimeout> | null = null;
const message = ref("");
const settingsOpen = ref(false);
const autostartEnabled = ref(false);
const hooksStatus = ref<"" | "loading" | "ok" | "error" | "cancelled">("");
const contextMenu = ref({ visible: false, x: 0, y: 0 });
const contextMenuEl = ref<HTMLElement | null>(null);
const appVersion = ref("");
const selectedCharacter = ref<Character>(
  (localStorage.getItem("selectedCharacter") as Character) ?? "zeztz",
);

const petAppearance = computed(() => {
  const isNotified = petState.value === "notified";
  if (selectedCharacter.value === "border-collie") {
    return isNotified ? dogNotifiedImg : dogIdleImg;
  }
  if (selectedCharacter.value === "rider1") {
    return isNotified ? rider1NotifiedImg : rider1IdleImg;
  }
  if (selectedCharacter.value === "v3") {
    return isNotified ? v3NotifiedImg : v3IdleImg;
  }
  return isNotified ? zeztzNotifiedImg : zeztzIdleImg;
});

function showContextMenu(event: MouseEvent) {
  contextMenu.value = { visible: true, x: event.clientX, y: event.clientY };
  nextTick(() => {
    if (!contextMenuEl.value) return;
    const { offsetWidth: w, offsetHeight: h } = contextMenuEl.value;
    contextMenu.value = {
      visible: true,
      x: Math.min(event.clientX, window.innerWidth - w),
      y: Math.min(event.clientY, window.innerHeight - h),
    };
  });
}

function hideContextMenu() {
  contextMenu.value.visible = false;
}

function selectCharacter(char: Character) {
  selectedCharacter.value = char;
  localStorage.setItem("selectedCharacter", char);
  hideContextMenu();
}

async function openSettings() {
  hideContextMenu();
  autostartEnabled.value = await invoke<boolean>("plugin:autostart|is_enabled");
  hooksStatus.value = "";
  settingsOpen.value = true;
}

async function setupHooks(scope: "global" | "project") {
  let projectPath: string | null = null;

  if (scope === "project") {
    const selected = await openDialog({ directory: true, multiple: false, title: "選擇專案資料夾" });
    if (!selected) {
      hooksStatus.value = "cancelled";
      setTimeout(() => { hooksStatus.value = ""; }, 2000);
      return;
    }
    projectPath = selected as string;
  }

  hooksStatus.value = "loading";
  try {
    await invoke("setup_claude_hooks", { projectPath });
    hooksStatus.value = "ok";
    setTimeout(() => { hooksStatus.value = ""; }, 3000);
  } catch {
    hooksStatus.value = "error";
    setTimeout(() => { hooksStatus.value = ""; }, 3000);
  }
}

async function toggleAutostart() {
  if (autostartEnabled.value) {
    await invoke("plugin:autostart|enable");
  } else {
    await invoke("plugin:autostart|disable");
  }
}

async function doInstallUpdate() {
  updating.value = true;
  try {
    await invoke("install_update");
  } catch {
    updating.value = false;
  }
}

onMounted(async () => {
  appVersion.value = await getVersion();

  await listen("claude-event", (event) => {
    console.log("收到 Claude 通知:", event.payload);
    if (notifiedTimer) clearTimeout(notifiedTimer);
    petState.value = "notified";
    message.value = "Claude 任務完成！";
    notifiedTimer = setTimeout(() => {
      petState.value = "pending";
    }, 5000);
  });

  await listen("claude-start", () => {
    if (notifiedTimer) clearTimeout(notifiedTimer);
    petState.value = "idle";
    message.value = "";
  });

  await listen<UpdateInfo>("update-available", (event) => {
    updateInfo.value = event.payload;
  });

  await listen<{ manual: boolean }>("update-not-available", (event) => {
    if (event.payload.manual) showUpToDateToast();
  });

  await listen("update-error", () => {
    showUpToDateToast();
  });
});
</script>

<style scoped>
.pet-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  padding-bottom: 20px;
  user-select: none;
  -webkit-user-select: none;
}

.pet-wrapper {
  position: relative;
  display: inline-flex;
}

@keyframes badge-float {
  0%, 100% { transform: translateX(-50%) translateY(0); }
  50% { transform: translateX(-50%) translateY(-6px); }
}

.pending-badge {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  width: 36px;
  height: 36px;
  object-fit: contain;
  z-index: 10;
  animation: badge-float 1s ease-in-out infinite;
  pointer-events: none;
}

.pet-character {
  width: 120px;
  height: 120px;
  object-fit: contain;
  transition: all 0.3s ease;
  pointer-events: none;
}

.pet-character.notified {
  transform: scale(1.3);
}


/* 右鍵選單 */
.context-menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 999;
}

.context-menu {
  position: fixed;
  background: #fff;
  border: 1px solid #ddd;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  min-width: 100px;
  z-index: 1000;
  overflow: hidden;
}

.context-menu-item {
  padding: 8px 16px;
  cursor: pointer;
  font-size: 13px;
  color: #333;
  user-select: none;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.context-menu-item:hover {
  background: #f0f0f0;
}

.context-menu-check {
  color: #333;
  font-size: 13px;
}

.context-menu-divider {
  height: 1px;
  background: #e8e8e8;
  margin: 2px 0;
}

.context-menu-version {
  padding: 5px 16px;
  font-size: 11px;
  color: #aaa;
  user-select: none;
}

/* 設定 Dialog */
.dialog-overlay {
  position: fixed;
  inset: 0;
  /* background: rgba(0, 0, 0, 0.4); */
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.dialog {
  background: #fff;
  border-radius: 10px;
  padding: 16px;
  width: 200px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.dialog-title {
  font-weight: bold;
  font-size: 14px;
  color: #333;
}

.toggle-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  color: #444;
}

.dialog-close {
  padding: 6px;
  border: 1px solid #ddd;
  border-radius: 6px;
  background: #f5f5f5;
  cursor: pointer;
  font-size: 12px;
}

.dialog-close:hover {
  background: #e8e8e8;
}

.hooks-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.hooks-label {
  font-size: 12px;
  color: #555;
}

.hooks-buttons {
  display: flex;
  gap: 6px;
}

.hooks-btn {
  flex: 1;
  padding: 5px 0;
  border: 1px solid #aaa;
  border-radius: 6px;
  background: #f0f0f0;
  cursor: pointer;
  font-size: 12px;
}

.hooks-btn:hover:not(:disabled) {
  background: #e0e0e0;
}

.hooks-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.hooks-ok {
  font-size: 12px;
  color: #4caf50;
}

.hooks-error {
  font-size: 12px;
  color: #f44336;
}

.hooks-loading {
  font-size: 12px;
  color: #888;
}

.hooks-hint {
  font-size: 12px;
  color: #999;
}

/* 更新通知 banner */
.update-banner {
  position: fixed;
  top: 8px;
  left: 8px;
  right: 8px;
  background: rgba(30, 100, 200, 0.92);
  color: #fff;
  border-radius: 8px;
  padding: 8px 10px;
  font-size: 11px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  z-index: 3000;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.update-actions {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}

.update-btn {
  background: #fff;
  color: #1e64c8;
  border: none;
  border-radius: 4px;
  padding: 3px 8px;
  font-size: 11px;
  cursor: pointer;
  font-weight: bold;
}

.update-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.update-dismiss {
  background: transparent;
  color: rgba(255, 255, 255, 0.7);
  border: none;
  cursor: pointer;
  font-size: 12px;
  padding: 0 2px;
}

.update-toast {
  position: fixed;
  top: 8px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(40, 40, 40, 0.88);
  color: #fff;
  border-radius: 6px;
  padding: 5px 12px;
  font-size: 11px;
  z-index: 3000;
  white-space: nowrap;
  pointer-events: none;
}
</style>
