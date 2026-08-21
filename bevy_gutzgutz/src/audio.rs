//! `GutzAudioPlugin`（骨組みのみ）。
//!
//! サウンドが必要なゲームが出てから中身を詰める。実装する際、音量設定
//! （マスター/BGM/SE等）はこのプラグインのResourceとして持たせつつ、
//! ディスクへの読み書きは[`crate::save::GutzSavePlugin`]ではなくBevy 0.19の
//! 公式`bevy::settings`（`SettingsPlugin`）へ委ねること——音量は
//! [`crate::save`]モジュールdocに書いた通り「ゲームの進行データ」ではなく
//! 「プレイヤー環境の好み（設定）」であり、gutzgutz側に専用の永続化機構を
//! 重ねて作る意味がない（Bevy本体を薄くラップしない、という既存方針と
//! 同じ理由）。

use bevy::prelude::*;

pub struct GutzAudioPlugin;

impl Plugin for GutzAudioPlugin {
    fn build(&self, _app: &mut App) {}
}
