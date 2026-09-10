# Desktop Pet 使用指南

## 啟動

直接開啟 Desktop Pet App。從原始碼執行（需先備妥 Node.js、pnpm、Rust 與 Tauri 建置環境）：

```sh
pnpm install
pnpm tauri dev
```

macOS 可用 `make preview` 建置並開啟 App，產物位於 `target/release/bundle/macos/Desktop Pet.app`。

## 寵物放哪裡

從選單列／系統匣選 **Open Pets Folder**，或直接開啟：

| 系統 | 路徑 |
| --- | --- |
| macOS／Linux | `~/.local/share/desktop-pet/pets/` |
| Windows | `%USERPROFILE%\.local\share\desktop-pet\pets\` |

## 怎麼放

1. 解壓縮寵物包，把**包含 `pet.json` 的整個資料夾**放入 `pets/`。
2. 選 **Reload Content** 載入。
3. 選 **Next Pet** 切換到新寵物。

```text
pets/
└── my_pet/
    ├── pet.json
    └── animations/
        └── idle/
            ├── 001.png
            ├── 002.png
            └── 003.png
```

**每隻寵物一個資料夾，`pet.json` 必須在 `pets/寵物資料夾/` 下，不可多包一層。** 多隻寵物的 `id` 不可重複。更新檔案後要再選 Reload Content；移除時把整個寵物資料夾移出，再重新載入。

## 只有圖片，怎麼做成寵物包

1. 按上圖建立資料夾，把 PNG 放入 `animations/idle/`。
2. 複製 [最小設定](examples/minimal.pet.json) 到 `my_pet/`，檔名改為 `pet.json`。
3. 修改需要的欄位，儲存後 Reload Content → Next Pet。

| 欄位 | 用途 |
| --- | --- |
| `id`、`display_name` | 唯一識別名稱、選單顯示名稱。 |
| `render.scale` | 顯示倍率，例如 `0.5` 縮小一半。 |
| `animations.idle.fps` | 每秒播放格數，越大越快。 |
| `animations.idle.source.dir` | 相對於 `pet.json` 的圖片路徑，Windows 也使用 `/`。 |

PNG 使用透明背景；同一隻寵物所有影格尺寸必須相同，寬高各 1～4096 像素。檔名依數字排序，一張圖也能使用。GIF、影片與 spritesheet 需先拆成一張一格的 PNG。最小設定只循環播放待機動畫；完整欄位見 [JSON Schema](pet.schema.json)。

## 操作與設定

| 操作 | 說明 |
| --- | --- |
| 左鍵拖曳角色 | 移動寵物，放開後保存位置；透明處可穿透點擊。 |
| Next Pet | 切換寵物。 |
| Reload Content | 套用新增或修改的寵物檔案，通常保留目前寵物。 |
| Exit | 保存位置並退出。 |
| 滑鼠停在角色上 | 顯示 Codex 用量與重設時間；需 Codex 已安裝並登入。百分比是已使用量。 |

- **調整大小／重設位置：** 先 Exit，再編輯 `pets/` 上一層的 `settings.json`。`scale` 調整全部寵物大小；`last_position` 改成 `null` 可重設位置，重新啟動後生效。
- **寵物沒出現：** 檢查放置層級、檔名是否為 `pet.json`、圖片路徑及重複 `id`，再 Reload Content → Next Pet。沒有可用寵物時會顯示 Debug Pet。
- **查看錯誤：** 開啟 `pets/` 上一層的 `logs/latest.log`，重新載入後查看底部紀錄。
