# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 專案簡介

**claude-pet** 是一個桌面寵物應用程式，基於 Tauri 2 + Vue 3 + TypeScript 建構。視窗為 250×250、透明、無邊框、永遠置頂，啟動時自動定位到螢幕右下角（Windows 工作列上方）。當外部工具（如 Claude Code）完成任務後，可透過 HTTP Webhook 通知寵物播放動畫反應。

## 常用指令

```bash
# 啟動開發模式（同時啟動 Rust backend + Vite frontend）
pnpm tauri dev

# 只啟動前端 Vite dev server（不啟動 Rust）
pnpm dev

# 型別檢查 + 建置前端
pnpm build

# 打包成桌面安裝檔
pnpm tauri build

# 測試 Webhook（應用程式執行中時）
curl -X POST http://127.0.0.1:9527/claude-notify -H "Content-Type: application/json" -d "{\"done\": true}"
```

## 架構說明

### 雙進程架構

```
外部工具
  │  HTTP POST /claude-notify
  ▼
Axum HTTP 伺服器（port 9527）   ← src-tauri/src/main.rs
  │  app.emit("claude-event", payload)
  ▼
Tauri 事件系統
  │  listen("claude-event", ...)
  ▼
Vue 前端（src/App.vue）
  └─ petState → 'notified'（5 秒後自動恢復 'idle'）
```

### 前端（`src/App.vue`）

唯一的 Vue 元件，包含以下狀態與功能：

- **`petState`**：`'idle'` / `'notified'`，收到 Webhook 後切換為 `'notified'`，5 秒後恢復
- **`selectedCharacter`**：`'zeztz'` / `'border-collie'`，持久化於 `localStorage`
- **`petAppearance`**：computed，根據 `selectedCharacter` + `petState` 決定顯示哪張圖片
- **右鍵選單**：切換角色（Zeztz / Border Collie）、開啟設定
- **設定 Dialog**：開機自動啟動切換（呼叫 `tauri-plugin-autostart` 指令）

角色圖片資源位於 `src/assets/imgs/`，各角色有 `idle` 和 `notified` 兩種狀態圖片：
- `Zeztz/zeztz-idle.png`、`Zeztz/zeztz-nofified.png`（注意：檔名有 typo）
- `Border Collie/dog-idle.png`、`Border Collie/dog-notified.png`

### Rust 後端（`src-tauri/src/main.rs`）

應用程式的實際入口點（`lib.rs` 僅為未使用的佔位檔案）：

1. **視窗定位**：`setup` 中讀取螢幕解析度，將視窗定位到右下角（預留 48px 工作列高度）
2. **系統匣圖示**：左鍵點擊切換視窗顯示/隱藏，右鍵選單有「隱藏寵物」與「退出」
3. **Axum HTTP 伺服器**：以 `tauri::async_runtime::spawn` 在背景啟動，監聽 port 9527，收到 POST 後呼叫 `app.emit("claude-event", payload)` 轉發給前端

### 視窗設定（`src-tauri/tauri.conf.json`）

`transparent: true`、`decorations: false`、`alwaysOnTop: true`、`shadow: false`、`skipTaskbar: true`。修改外觀時需確認 CSS 的 `background` 配合透明（目前無背景色設定）。

## 技術堆疊

| 層級 | 技術 |
|------|------|
| 前端框架 | Vue 3 (`<script setup>` SFC) |
| 語言 | TypeScript (前端) / Rust 2021 (後端) |
| 桌面殼層 | Tauri 2 |
| HTTP 伺服器 | Axum 0.7 + Tokio |
| 建置工具 | Vite 6（前端固定 port 1420） |
| 套件管理 | pnpm |
| 自動啟動 | tauri-plugin-autostart |
