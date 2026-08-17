# Funkoro

Funkoro のゲーム本体。共通機能は同梱の `bevy_gutzgutz` を利用するが、ゲーム固有の
State・ルール・画面・アセットはこのクレートで管理する。

## 現在の試作

フンコロガシの一人称視点で糞玉を巣穴へ押し運ぶ、3ステージの玉ころがしパズル。
外部物理エンジンには依存せず、玉の慣性・外周・岩柱の反発だけを軽量な専用ロジックで
扱う。`GutzGameSessionPlugin` により、ゲーム状態、Action入力、UI画面スタック、
最高到達ステージの保存を接続している。

- `Enter` / `Space` — 開始・再挑戦
- `WASD` — 移動
- マウス / `Q` `E` — 見回す
- `R` — 現在のステージをやり直す

## 開発コマンド

```powershell
cargo run --features dev  # 高速リンク + devtools
cargo check-all
cargo test-all
cargo lint
cargo fmt-check
```

通常の `cargo build --release` には開発用の動的リンクとdevtoolsを含めない。

## 構成

- `src/` — ゲーム固有コード
- `assets/` — ゲーム固有アセット（追加時に作成）
- `bevy_gutzgutz/` — 共通基盤。ゲーム固有の実装を追加しない
- `.cargo/config.toml` — チーム共通のCargo alias
