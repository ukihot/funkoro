//! フンコロガシの身体と球が一体で変化する、一人称の玉ころがしパズル。

mod play;
mod scene;
mod ui;

use bevy::prelude::*;
use bevy_gutzgutz::{
    input::Actionlike,
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

#[derive(Actionlike, Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
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

#[derive(Clone, Copy)]
pub(super) struct HoleLayout {
    pub(super) stage: u8,
    pub(super) position: Vec3,
    pub(super) radius: f32,
}

#[derive(Resource)]
pub(super) struct RunLayout {
    pub(super) random_state: u64,
    pub(super) hole: Option<HoleLayout>,
}

impl Default for RunLayout {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9_7F4A_7C15, |time| time.as_nanos() as u64);
        Self { random_state: seed, hole: None }
    }
}

#[derive(Resource, Default)]
pub(super) struct ClearScore {
    pub(super) first_clear_diameter_cm: Option<u32>,
}

/// ゲーム固有の層だけを組み立てる。共通のセッション機能は gutzgutz が受け持つ。
pub(crate) struct FunkoroGamePlugin;

impl Plugin for FunkoroGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<PuzzleProgress>()
            .init_resource::<RunLayout>()
            .init_resource::<ClearScore>()
            .add_plugins(
                GutzGameSessionPlugin::<GameState, BeetleAction, UiScreen, SaveData>::standard_save_location(
                    "com", "gutzgutz", "funkoro", "save.toml",
                ),
            )
            .add_systems(
                Startup,
                (ui::configure_controls, scene::setup_world, ui::setup_hud, ui::open_title_screen).chain(),
            )
            .add_systems(
                Update,
                (play::open_menu, play::update_beetle, play::roll_ball, play::reset_stage, ui::update_hud)
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
