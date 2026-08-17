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
    BeetleAction, ClearScore, GameState, PuzzleProgress, RunLayout, UiScreen,
    play::{place_stage, view_pressure},
    scene::{Ball, Beetle, LookAngles, Nest},
};

#[derive(Component)]
pub(super) struct ScreenUi;

#[derive(Component)]
pub(super) struct HoleHud;

#[derive(Component)]
pub(super) struct TiltHud;

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

pub(super) fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Node { position_type: PositionType::Absolute, top: px(18), left: px(20), ..default() },
        Text::new(""),
        TextFont { font_size: FontSize::Px(22.0), ..default() },
        TextColor(Color::srgb(1.0, 0.88, 0.62)),
        Visibility::Hidden,
        HoleHud,
    ));
    commands.spawn((
        Node { position_type: PositionType::Absolute, top: px(18), right: px(20), ..default() },
        Text::new(""),
        TextFont { font_size: FontSize::Px(22.0), ..default() },
        TextColor(Color::srgb(1.0, 0.88, 0.62)),
        Visibility::Hidden,
        TiltHud,
    ));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn start_from_menu(
    actions: Res<GutzActionState<BeetleAction>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
    mut progress: ResMut<PuzzleProgress>,
    mut run_layout: ResMut<RunLayout>,
    mut clear_score: ResMut<ClearScore>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<(&mut Transform, &mut Nest), (Without<Ball>, Without<Beetle>)>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
    mut cursor: Single<&mut CursorOptions>,
    existing: Query<Entity, With<ScreenUi>>,
    mut commands: Commands,
    mut hud: Query<&mut Visibility, Or<(With<HoleHud>, With<TiltHud>)>>,
) {
    if !actions.just_pressed(BeetleAction::Start) {
        return;
    }
    if *state.get() != GameState::Menu {
        progress.stage = 0;
        clear_score.first_clear_diameter_cm = None;
        place_stage(&progress, &mut run_layout, true, &mut ball, &mut nest, &mut beetle);
    }
    screens.clear();
    next_state.set(GameState::Playing);
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
    for mut visibility in &mut hud {
        *visibility = Visibility::Visible;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn release_cursor(
    mut cursor: Single<&mut CursorOptions>,
    mut hud: Query<&mut Visibility, Or<(With<HoleHud>, With<TiltHud>)>>,
) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
    for mut visibility in &mut hud {
        *visibility = Visibility::Hidden;
    }
}

pub(super) fn update_hud(
    nest: Single<&Nest, (Without<Ball>, Without<Beetle>)>,
    ball: Single<&Ball, Without<Beetle>>,
    mut hole_text: Query<&mut Text, (With<HoleHud>, Without<TiltHud>)>,
    mut tilt_text: Query<&mut Text, (With<TiltHud>, Without<HoleHud>)>,
) {
    let diameter_cm = (nest.hole_radius * 200.0).round() as u32;
    **hole_text.single_mut().expect("hole HUD is spawned during startup") =
        format!("HOLE {diameter_cm} cm");
    let tilt_percent = (view_pressure(ball.radius) * 100.0).round() as u32;
    **tilt_text.single_mut().expect("tilt HUD is spawned during startup") =
        format!("TILT {tilt_percent}%");
}

pub(super) fn open_or_close_screen(
    mut opened: MessageReader<GutzUiScreenOpened<UiScreen>>,
    mut closed: MessageReader<GutzUiScreenClosed<UiScreen>>,
    mut commands: Commands,
    existing: Query<Entity, With<ScreenUi>>,
    clear_score: Res<ClearScore>,
) {
    if closed.read().next().is_some() {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
    }
    for GutzUiScreenOpened(screen) in opened.read() {
        let (title, body) = match screen {
            UiScreen::Title => ("FUNKORO", "THE SOIL REMEMBERS".to_owned()),
            UiScreen::Menu => ("PAUSED", String::new()),
            UiScreen::Cleared => (
                "NEST SEALED",
                clear_score
                    .first_clear_diameter_cm
                    .map_or_else(String::new, |diameter| format!("BALL {diameter} cm")),
            ),
            UiScreen::Stuck => ("STUCK", String::new()),
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
