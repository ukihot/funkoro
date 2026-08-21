use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
use leafwing_input_manager::prelude::{ActionState, Actionlike};

use crate::lifecycle::{GutzExecutionContext, GutzLifecycleState};

/// ゲーム側が定義する「プレイヤーが何をしたいか」を表すAction型が満たすべき
/// 制約をまとめたマーカートレイト。leafwing-input-managerの`Actionlike`
/// （`Reflect`実装が要る）をそのまま要求する——gutzgutzが独自の制約を
/// 追加で課すことはしない。
///
/// ```ignore
/// #[derive(Actionlike, Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
/// enum PlayerAction {
///     RotateLeft,
///     RotateRight,
///     Charge,
///     Restart,
/// }
/// impl GutzAction for PlayerAction {}
/// ```
pub trait GutzAction: Actionlike + Copy + GetTypeRegistration {}

impl<T: Actionlike + Copy + GetTypeRegistration> GutzAction for T {}

/// gutzgutzが内部で保持する、Action入力の実体（leafwingの`InputMap<A>`/
/// `ActionState<A>`）を持つ唯一のエンティティのマーカー。ゲーム側はこの
/// エンティティを`Single<&mut InputMap<A>>`（バインド設定時）や
/// `Single<&ActionState<A>>`（毎フレームの読み取り）でクエリする。
///
/// 複数エンティティに分散させない理由：gutzgutzが対象とするのは単一プレイヤー
/// の「今アクティブな入力」であり、マルチプレイヤーやAI操作エンティティ別の
/// Action等、per-entity化が必要な場面はleafwing自体を直接使う方が素直
/// （README.md「早すぎる抽象化はしない」）。
#[derive(Component)]
pub struct GutzInputEntity;

/// `action`が有効なのは`context`の間だけ、という制限の一覧。leafwing自体は
/// Actionの実行コンテキストという概念を持たないため、gutzgutzが足す
/// 唯一の付加価値がこれ（`crate::input`のモジュールdoc参照）。
///
/// 設定しないActionはどの実行コンテキストでも常に評価される。
#[derive(Resource)]
pub struct GutzInputContexts<A: GutzAction> {
    contexts: HashMap<A, GutzExecutionContext>,
}

impl<A: GutzAction> Default for GutzInputContexts<A> {
    fn default() -> Self {
        Self { contexts: HashMap::default() }
    }
}

impl<A: GutzAction> GutzInputContexts<A> {
    /// `action`が有効なのは`context`の間だけ、という制限を追加する
    /// （doc：「LifecycleによってActionの有効範囲を制御する」）。
    pub fn restrict_to(&mut self, action: A, context: GutzExecutionContext) -> &mut Self {
        self.contexts.insert(action, context);
        self
    }
}

/// `GutzInputContexts`の内容を、[`GutzInputEntity`]が持つ`ActionState<A>`へ
/// 毎フレーム反映する。leafwingの`ActionState::disable_action`/
/// `enable_action`をそのまま使い、gutzgutz独自の入力状態は一切保持しない
/// （旧実装の`GutzActionState`が持っていた`pressed`/`just_pressed`等の
/// HashSetを毎フレーム再計算する処理は、leafwing本体にそのまま委ねられる）。
///
/// leafwingの`InputManagerSystem::Update`（実入力→`ActionState`反映）の
/// 直後に実行する必要がある——先に無効化してしまうと、その後の
/// `update_action_state`が実入力で上書きしてしまい、コンテキスト制限が
/// 1フレーム遅れて効いているように見える。
pub(crate) fn apply_context_restrictions<A: GutzAction, S: GutzLifecycleState>(
    restrictions: Res<GutzInputContexts<A>>,
    current_state: Option<Res<State<S>>>,
    mut action_state: Single<&mut ActionState<A>, With<GutzInputEntity>>,
) {
    let context = current_state.map(|state| state.get().execution_context());
    for (action, required) in restrictions.contexts.iter() {
        if Some(*required) == context {
            action_state.enable_action(action);
        } else {
            action_state.disable_action(action);
        }
    }
}
