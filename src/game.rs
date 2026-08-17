//! フンコロガシの身体と球が一体で変化する、一人称の玉ころがしパズル。

mod play;
mod scene;
mod ui;

use bevy::prelude::*;
use bevy_gutzgutz::{
    lifecycle::{GutzExecutionContext, GutzLifecycleState, in_game},
    session::GutzGameSessionPlugin,
};
use serde::{Deserialize, Serialize};

pub(super) const ARENA_HALF_SIZE: f32 = 11.5;
pub(super) const BEETLE_RADIUS: f32 = 0.46;
pub(super) const START_BALL_RADIUS: f32 = 0.38;
pub(super) const MAX_BALL_RADIUS: f32 = 1.45;
pub(super) const STAGE_COUNT: u8 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
pub(super) enum GameState {
    #[default]
    Title,
    Playing,
    Menu,
    Cleared,
    Stuck,
}

impl GutzLifecycleState for GameState {
    fn execution_context(&self) -> GutzExecutionContext {
        match self {
            Self::Playing => GutzExecutionContext::InGame,
            Self::Title | Self::Menu | Self::Cleared | Self::Stuck => GutzExecutionContext::OutGame,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum BeetleAction {
    PushBackward,
    TurnLeft,
    TurnRight,
    ResetStage,
    OpenMenu,
    Start,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum UiScreen {
    Title,
    Menu,
    Cleared,
    Stuck,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SaveData {
    best_stage: u8,
}

#[derive(Resource, Default)]
pub(super) struct PuzzleProgress {
    stage: u8,
    best_stage: u8,
}

/// ゲーム固有の層だけを組み立てる。共通のセッション機能は gutzgutz が受け持つ。
pub(crate) struct FunkoroGamePlugin;

impl Plugin for FunkoroGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<PuzzleProgress>()
            .add_plugins(
                GutzGameSessionPlugin::<GameState, BeetleAction, UiScreen, SaveData>::standard_save_location(
                    "com", "gutzgutz", "funkoro", "save.toml",
                ),
            )
            .add_systems(
                Startup,
                (ui::configure_controls, scene::setup_world, ui::open_title_screen).chain(),
            )
            .add_systems(
                Update,
                (play::open_menu, play::update_beetle, play::roll_ball, play::reset_stage)
                    .chain()
                    .run_if(in_game::<GameState>()),
            )
            .add_systems(
                Update,
                (ui::release_cursor, ui::open_or_close_screen, ui::start_from_menu)
                    .chain()
                    .run_if(not(in_game::<GameState>())),
            );
    }
}
