# Pulsebar

Windows 11 のタスクバー上に CPU / メモリ使用率のミニメーターを表示する超軽量ユーティリティです。
*Pulsebar — a tiny CPU / RAM meter overlay for the Windows 11 taskbar.* — [English below](#english)

![screenshot](docs/screenshot.png)

## 特徴

- **タスクバー本体に表示** — 通知領域のすぐ左に、上段 = CPU / 下段 = メモリの 2 段バーを重ねて表示します
- **超軽量** — 実測でメモリ約 4 MB・CPU ほぼ 0%(更新は 1 秒間隔、`GetSystemTimes` / `GlobalMemoryStatusEx` のみ使用。WMI / パフォーマンスカウンター不使用)
- **依存なしの単一 exe** — 約 130 KB。ランタイムのインストール不要
- **左クリック**でタスクマネージャーを起動
- **右クリック**でメニュー(タスクマネージャー / スタートアップ登録 / 終了)
- **ホバー**で正確な値をツールチップ表示(CPU % とメモリの GB 表記)
- ライト / ダークテーマ・DPI 変更・Explorer 再起動に自動追従
- 使用率 80% 以上でバーが黄色、90% 以上で赤に変化

## インストール

[Releases](../../releases) から入手できます。2 つの形式があります。

- **インストーラー版(推奨)**: `pulsebar-setup-x.y.z.exe` を実行します。
  サインイン時の自動起動もセットアップ中に設定できます(管理者権限不要)。
  アンインストールは Windows の「設定 → アプリ」から行えます。
- **ポータブル版**: `pulsebar.exe` をダウンロードして実行するだけです。
  自動起動したい場合は、右クリックメニューの「Run at startup」を使うか、
  `Win + R` → `shell:startup` に exe のショートカットを置いてください。
  アンインストールは、右クリック → 「Exit」で終了して exe を削除するだけです。

> **Note**: 実行ファイルに署名を行っていないため、初回実行時に SmartScreen の
> 警告が表示されることがあります。「詳細情報」→「実行」で起動できます。

## 使い方

| 操作 | 動作 |
|---|---|
| 左クリック | タスクマネージャーを起動 |
| 右クリック | メニュー(スタートアップ登録 / 終了) |
| ホバー | 詳細値のツールチップ |

## 仕組み

Windows 11 では DeskBand(タスクバー拡張の公式 API)が廃止されたため、
本アプリは `Shell_TrayWnd`(タスクバー)を探して自前のレイヤードウィンドウを
`SetParent` で子ウィンドウとして取り付け、per-pixel alpha
(`UpdateLayeredWindow`)で描画しています(TrafficMonitor と同様の方式)。

## 制限事項

- **非公式な手法です。** Windows Update でタスクバーの内部構造が変わると表示できなくなる可能性があります。その場合は自動的に「互換モード」(画面右下の最前面ミニウィンドウ)に切り替わります。
- タスクバー中央寄せのアプリアイコンが非常に多い場合、アイコンがウィジェットの下に潜って見えることがあります。
- プライマリモニターのタスクバーのみ対応(セカンダリタスクバーには表示されません)。
- ExplorerPatcher 等のシェル改変ツールとの併用は動作対象外です。

## ビルド

Rust(stable)と Visual Studio Build Tools(MSVC)が必要です。

```
cargo build --release
```

`target\release\pulsebar.exe` が生成されます。

---

## English

Pulsebar is a tiny CPU / RAM meter that sits directly on the Windows 11
taskbar, just left of the notification area. Top bar = CPU, bottom bar = RAM —
your system's pulse, always visible.

- **Ultra lightweight**: ~4 MB RAM, ~0% CPU (1 s updates via `GetSystemTimes` / `GlobalMemoryStatusEx`; no WMI, no perf counters)
- **Single dependency-free exe** (~130 KB), no runtime required
- **Left-click** opens Task Manager, **right-click** shows a menu (run at startup / exit), **hover** shows exact values
- Follows light/dark theme, DPI changes, and Explorer restarts
- Bars turn yellow at ≥80% and red at ≥90%

**Install**: grab either flavor from [Releases](../../releases) —
`pulsebar-setup-x.y.z.exe` (installer; per-user, no admin rights, optional
run-at-sign-in, uninstall via Settings → Apps) or the portable `pulsebar.exe`
(just run it; to uninstall, right-click → Exit and delete the exe). The
binaries are unsigned, so SmartScreen may warn on first launch — choose
"More info" → "Run anyway".

**How it works**: Windows 11 removed the official DeskBand API, so Pulsebar
finds `Shell_TrayWnd`, attaches its own layered window via `SetParent`, and
renders with per-pixel alpha through `UpdateLayeredWindow` (the same
technique used by TrafficMonitor).

**Limitations**: this is an unofficial technique — a Windows update that
changes the taskbar internals may break it, in which case the app falls back
to a small always-on-top window near the bottom-right corner. Primary
monitor only. Not supported alongside shell mods such as ExplorerPatcher.

**Build**: `cargo build --release` (stable Rust + MSVC).

## License

[MIT](LICENSE)
