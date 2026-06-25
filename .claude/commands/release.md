# claude-pet 一鍵發版

發布 claude-pet 新版本。自動同步三個設定檔的版本號，建立 git commit 與 tag，推送至 GitHub 以觸發 CI/CD 自動建置與發布。

## 使用方式

```
/release [patch|minor|major|X.Y.Z]
```

- 無參數：自動分析上次 tag 後的 commits，判斷 patch / minor / major
- `patch`：強制遞增修訂版號（例：0.2.1 → 0.2.2）
- `minor`：強制遞增次版本號（例：0.2.1 → 0.3.0）
- `major`：強制遞增主版本號（例：0.2.1 → 1.0.0）
- `X.Y.Z`：直接指定版本號

## 執行步驟

### 步驟 1：確認 working tree 乾淨

執行 `git status` 確認沒有未提交的修改。若有未提交的變更，詢問使用者是否要繼續（建議先 commit 或 stash）。

### 步驟 2：讀取目前版本

讀取 `package.json` 的 `"version"` 欄位取得目前版本號。

### 步驟 3：計算新版本號

若 `$ARGUMENTS` 有值，直接套用：
- `patch`：patch +1
- `minor`：minor +1，patch 歸零
- `major`：major +1，minor/patch 歸零
- `X.Y.Z` 格式：直接使用

若 `$ARGUMENTS` **為空**，自動分析 commits 判斷：

1. 執行 `git log {上次tag}..HEAD --oneline` 取得自上次 tag 後的所有 commit 訊息
2. 依 Conventional Commits 規則判斷：
   - 任一 commit 含 `BREAKING CHANGE` 或 `!`（如 `feat!:`）→ **major**
   - 任一 commit 以 `feat:` 或 `feat(` 開頭 → **minor**
   - 其餘（`fix:`、`chore:`、`refactor:` 等）→ **patch**
3. 顯示判斷依據，例如：「根據 3 個 commits 分析，判斷為 minor（含新功能）」

向使用者確認：「即將從 {舊版本} 發布 v{新版本}（{判斷依據}），確認繼續？」

### 步驟 4：同步更新三個版本號

修改以下三個檔案：

1. **`package.json`**：`"version": "舊版本"` → `"version": "新版本"`
2. **`src-tauri/Cargo.toml`**：`[package]` 區塊的 `version = "舊版本"` → `version = "新版本"`
3. **`src-tauri/tauri.conf.json`**：頂層的 `"version": "舊版本"` → `"version": "新版本"`

顯示變更摘要確認三個檔案都已更新。

### 步驟 5：執行 git 操作

依序執行（每步確認輸出正常）：

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: release v{新版本}"
git tag v{新版本}
git push --follow-tags
```

### 步驟 6：完成提示

顯示成功訊息：
- 已發布版本：v{新版本}
- GitHub Actions 已觸發，建置 Windows 安裝檔約需 10-15 分鐘
- 建置完成後，Release 與 `latest.json` 會自動出現於 https://github.com/Thrizzacode/claude-pet/releases
- App 的自動更新機制即可偵測到此新版本
