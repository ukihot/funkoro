//! `GutzInputPlugin` — leafwing-input-managerを土台にした、Action入力の
//! 実行コンテキスト制限だけを足す薄いレイヤー。
//!
//! かつては`keyboard.pressed(KeyCode::KeyW)`のような生入力を自前でラップし、
//! Action/Device分離・複数デバイスの束ね・TOML設定読み込みを丸ごと
//! 手書きしていたが、その大半はleafwing-input-managerが既に高品質に
//! 解決している領域だった（`ActionState<A>::pressed`/`InputMap<A>::insert`
//! でほぼ同じことができる上、複数デバイスの同時バインドやclash解決は
//! むしろ向こうの方が作り込まれている）。Bevy/Avian3D本体を薄くラップ
//! しない、というgutzgutzの既存方針を`input`自体にも適用し、gutzgutzが
//! 独自に持つのは以下の1点だけにした：
//!
//! - [`crate::lifecycle`]によってActionの有効範囲（実行コンテキスト）を
//!   制御する（[`GutzInputContexts::restrict_to`]）。leafwing自体は
//!   「このActionはタイトル画面では無効」のような概念を持たない。
//!
//! ゲーム側はleafwingの型（[`InputMap`]/[`ActionState`]、いずれも
//! [`GutzInputEntity`]という唯一のエンティティが持つComponent）を
//! `Single<&mut InputMap<A>>`（バインド設定）/`Single<&ActionState<A>>`
//! （毎フレームの読み取り）でそのまま使う。gutzgutzは`leafwing_input_manager`
//! の型と[`Actionlike`]をそのまま再エクスポートしているため、通常の`use`は
//! `bevy_gutzgutz::input::{...}`だけで済む（`steam`モジュールが
//! `bevy-steamworks`を`sdk`として再エクスポートするのと同じ形）。
//!
//! **ただし1点だけ例外がある**：`#[derive(Actionlike)]`はコード生成時に
//! `leafwing_input_manager`という名前が**呼び出し元クレート自身の
//! `Cargo.toml`に直接依存として書かれているか**を`proc-macro-crate`経由で
//! 検出する（再エクスポート経由か、他クレート越しの間接依存かは見ない）。
//! そのためAction型を定義するゲーム側のcrateは、`leafwing-input-manager`を
//! 薄い直接依存として`Cargo.toml`に持つ必要がある（`default-features = false`
//! でよい——featureはgutzgutz側の指定とCargoのfeature統合により自動的に
//! 揃う）。これはgutzgutzの設計判断ではなく、`proc-macro-crate`ベースの
//! deriveマクロに共通する制約。

mod config;
mod map;
mod source;

use core::marker::PhantomData;

use bevy::prelude::*;
pub use leafwing_input_manager::Actionlike;
use leafwing_input_manager::plugin::{InputManagerPlugin, InputManagerSystem};
pub use leafwing_input_manager::prelude::{ActionState, InputMap};

pub use self::config::{load_into, load_into_from_file};
pub use self::map::{GutzAction, GutzInputContexts, GutzInputEntity};

use crate::lifecycle::GutzLifecycleState;

/// Action型`A`とゲームのState型`S`を受け取り、leafwingの`InputManagerPlugin<A>`
/// を配線した上で、[`GutzInputEntity`]（`InputMap<A>`/`ActionState<A>`を持つ
/// 唯一のエンティティ）を用意し、実行コンテキスト制限を毎フレーム適用する。
///
/// ```ignore
/// app.add_plugins(GutzInputPlugin::<PlayerAction, GameState>::default());
///
/// fn configure_controls(mut input_map: Single<&mut InputMap<PlayerAction>>) {
///     input_map.insert(PlayerAction::Fire, KeyCode::Space);
/// }
///
/// fn shoot(actions: Single<&ActionState<PlayerAction>>) {
///     if actions.just_pressed(&PlayerAction::Fire) { /* ... */ }
/// }
/// ```
pub struct GutzInputPlugin<A: GutzAction, S: GutzLifecycleState>(PhantomData<fn() -> (A, S)>);

impl<A: GutzAction, S: GutzLifecycleState> Default for GutzInputPlugin<A, S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: GutzAction, S: GutzLifecycleState> Plugin for GutzInputPlugin<A, S> {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<A>::default())
            .init_resource::<GutzInputContexts<A>>()
            .add_systems(
                PreUpdate,
                map::apply_context_restrictions::<A, S>.after(InputManagerSystem::Update),
            );

        // ゲームのStartupシステム（例：`configure_controls`）が
        // `Single<&mut InputMap<A>>`で確実にこのエンティティを見つけられるよう、
        // Startupスケジュールより前——`Plugin::build`の時点で同期的にspawnする
        // （旧`init_resource`が常にimmediateだったのと同じ即時性を保つため）。
        app.world_mut().spawn((
            GutzInputEntity,
            InputMap::<A>::default(),
            ActionState::<A>::default(),
        ));
    }
}
