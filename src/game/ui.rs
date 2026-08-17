use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_gutzgutz::{
    input::{GutzActionState, GutzInputMap, GutzInputSource},
    lifecycle::GutzExecutionContext,
    ui::{
        GutzModalPanelStyle, GutzUiScreenClosed, GutzUiScreenOpened, GutzUiStack, spawn_modal_panel,
    },
};

use super::{
    BeetleAction, GameState, PuzzleProgress, UiScreen,
    play::place_stage,
    scene::{Ball, Beetle, LookAngles, Nest},
};

#[derive(Component)]
pub(super) struct ScreenUi;

pub(super) fn configure_controls(mut input_map: ResMut<GutzInputMap<BeetleAction>>) {
    use BeetleAction::*;
    use GutzInputSource::Key;

    for (action, key) in [
        (PushBackward, KeyCode::KeyS),
        (TurnLeft, KeyCode::KeyQ),
        (TurnRight, KeyCode::KeyE),
        (ResetStage, KeyCode::KeyR),
        (OpenMenu, KeyCode::Escape),
    ] {
        input_map.bind(action, Key(key)).restrict_to(action, GutzExecutionContext::InGame);
    }
    input_map
        .bind(Start, Key(KeyCode::Enter))
        .bind(Start, Key(KeyCode::Space))
        .restrict_to(Start, GutzExecutionContext::OutGame);
}

pub(super) fn open_title_screen(mut screens: ResMut<GutzUiStack<UiScreen>>) {
    screens.push(UiScreen::Title);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn start_from_menu(
    actions: Res<GutzActionState<BeetleAction>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
    mut progress: ResMut<PuzzleProgress>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<(&mut Transform, &mut Nest), (Without<Ball>, Without<Beetle>)>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
    mut cursor: Single<&mut CursorOptions>,
    existing: Query<Entity, With<ScreenUi>>,
    mut commands: Commands,
) {
    if !actions.just_pressed(BeetleAction::Start) {
        return;
    }
    if *state.get() != GameState::Menu {
        progress.stage = 0;
        place_stage(&progress, &mut ball, &mut nest, &mut beetle);
    }
    screens.clear();
    next_state.set(GameState::Playing);
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
    for entity in &existing {
        commands.entity(entity).despawn();
    }
}

pub(super) fn release_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

pub(super) fn open_or_close_screen(
    mut opened: MessageReader<GutzUiScreenOpened<UiScreen>>,
    mut closed: MessageReader<GutzUiScreenClosed<UiScreen>>,
    mut commands: Commands,
    existing: Query<Entity, With<ScreenUi>>,
) {
    if closed.read().next().is_some() {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
    }
    for GutzUiScreenOpened(screen) in opened.read() {
        let (title, body) = match screen {
            UiScreen::Title => ("FUNKORO", "THE SOIL REMEMBERS"),
            UiScreen::Menu => ("PAUSED", ""),
            UiScreen::Cleared => ("NEST SEALED", ""),
            UiScreen::Stuck => ("STUCK", ""),
        };
        spawn_modal_panel(&mut commands, GutzModalPanelStyle::default())
            .insert(ScreenUi)
            .with_children(|panel| {
                panel.spawn((
                    Text::new(title),
                    TextFont { font_size: FontSize::Px(56.0), ..default() },
                    TextColor(Color::srgb(1.0, 0.82, 0.38)),
                ));
                if !body.is_empty() {
                    panel.spawn((
                        Text::new(body),
                        TextFont { font_size: FontSize::Px(22.0), ..default() },
                        TextColor(Color::srgb(1.0, 0.93, 0.78)),
                    ));
                }
            });
    }
}
