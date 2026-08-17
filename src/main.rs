//! フンコロガシの目線で、糞玉を巣穴まで押し運ぶ一人称パズルの試作。
//!
//! 外部の物理エンジンは使わず、玉に必要な慣性・壁・障害物との反発だけを
//! 小さな専用シミュレーションとして実装している。ゲーム固有のルールを軽く
//! 保ったまま、セッション、Action 入力、UI スタック、セーブは gutzgutz に委ねる。

use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_gutzgutz::{
    input::{GutzActionState, GutzInputMap, GutzInputSource},
    lifecycle::{GutzExecutionContext, GutzLifecycleState, in_game},
    save::GutzSaveRequest,
    session::GutzGameSessionPlugin,
    ui::{
        GutzModalPanelStyle, GutzUiScreenClosed, GutzUiScreenOpened, GutzUiStack, spawn_modal_panel,
    },
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "dev")]
use bevy_gutzgutz::devtools::GutzDevtoolsPlugin;

const ARENA_HALF_SIZE: f32 = 11.5;
const BALL_RADIUS: f32 = 0.72;
const BEETLE_RADIUS: f32 = 0.46;
const STAGE_COUNT: u8 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
enum GameState {
    #[default]
    Title,
    Playing,
    Cleared,
}

impl GutzLifecycleState for GameState {
    fn execution_context(&self) -> GutzExecutionContext {
        match self {
            Self::Playing => GutzExecutionContext::InGame,
            Self::Title | Self::Cleared => GutzExecutionContext::OutGame,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BeetleAction {
    Forward,
    Backward,
    Left,
    Right,
    TurnLeft,
    TurnRight,
    ResetStage,
    Start,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum UiScreen {
    Title,
    Cleared,
}

#[derive(Debug, Deserialize, Serialize)]
struct SaveData {
    best_stage: u8,
}

#[derive(Component)]
struct Beetle;

#[derive(Component)]
struct Ball {
    velocity: Vec3,
}

#[derive(Component)]
struct Nest;

#[derive(Component)]
struct Obstacle {
    half_extent: Vec2,
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct ScreenUi;

#[derive(Component)]
struct LookAngles {
    yaw: f32,
    pitch: f32,
}

#[derive(Resource, Default)]
struct PuzzleProgress {
    stage: u8,
    best_stage: u8,
}

fn main() {
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "FUNKORO".into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .init_resource::<PuzzleProgress>()
        .add_plugins(GutzGameSessionPlugin::<GameState, BeetleAction, UiScreen, SaveData>::standard_save_location(
            "com", "gutzgutz", "funkoro", "save.toml",
        ))
        .add_systems(Startup, (configure_controls, setup_world, setup_hud, open_title_screen).chain())
        .add_systems(
            Update,
            (
                start_from_menu,
                update_player,
                roll_ball,
                update_hud,
                reset_stage,
            )
                .run_if(in_game::<GameState>()),
        )
        .add_systems(Update, (open_or_close_screen, start_from_menu).run_if(not(in_game::<GameState>())));

    // 開発ビルドだけで有効にする。製品ビルドにはeguiやデバッグ操作を含めない。
    #[cfg(feature = "dev")]
    app.add_plugins(GutzDevtoolsPlugin);

    app.run();
}

fn configure_controls(mut input_map: ResMut<GutzInputMap<BeetleAction>>) {
    use BeetleAction::*;
    use GutzInputSource::Key;

    for (action, key) in [
        (Forward, KeyCode::KeyW),
        (Backward, KeyCode::KeyS),
        (Left, KeyCode::KeyA),
        (Right, KeyCode::KeyD),
        (TurnLeft, KeyCode::KeyQ),
        (TurnRight, KeyCode::KeyE),
        (ResetStage, KeyCode::KeyR),
    ] {
        input_map.bind(action, Key(key)).restrict_to(action, GutzExecutionContext::InGame);
    }
    input_map
        .bind(Start, Key(KeyCode::Enter))
        .bind(Start, Key(KeyCode::Space))
        .restrict_to(Start, GutzExecutionContext::OutGame);
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ClearColor(Color::srgb(0.055, 0.032, 0.018)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.95, 0.68, 0.4),
        brightness: 350.0,
        ..default()
    });

    let sand = materials.add(Color::srgb(0.31, 0.16, 0.055));
    let wall = materials.add(Color::srgb(0.12, 0.055, 0.018));
    let obstacle = materials.add(Color::srgb(0.22, 0.1, 0.026));
    let dung = materials.add(Color::srgb(0.13, 0.055, 0.014));
    let nest = materials.add(Color::srgb(0.015, 0.008, 0.003));

    commands.spawn((
        Mesh3d(
            meshes
                .add(Plane3d::default().mesh().size(ARENA_HALF_SIZE * 2.0, ARENA_HALF_SIZE * 2.0)),
        ),
        MeshMaterial3d(sand),
    ));

    for (position, size) in [
        (Vec3::new(0.0, 0.4, -ARENA_HALF_SIZE), Vec3::new(ARENA_HALF_SIZE * 2.0, 0.8, 0.4)),
        (Vec3::new(0.0, 0.4, ARENA_HALF_SIZE), Vec3::new(ARENA_HALF_SIZE * 2.0, 0.8, 0.4)),
        (Vec3::new(-ARENA_HALF_SIZE, 0.4, 0.0), Vec3::new(0.4, 0.8, ARENA_HALF_SIZE * 2.0)),
        (Vec3::new(ARENA_HALF_SIZE, 0.4, 0.0), Vec3::new(0.4, 0.8, ARENA_HALF_SIZE * 2.0)),
    ] {
        spawn_box(&mut commands, &mut meshes, &wall, position, size, None);
    }

    for (position, size) in [
        (Vec3::new(-2.8, 0.65, -1.7), Vec3::new(1.4, 1.3, 4.8)),
        (Vec3::new(2.7, 0.65, 2.5), Vec3::new(4.6, 1.3, 1.3)),
        (Vec3::new(4.3, 0.65, -4.0), Vec3::new(1.2, 1.3, 3.2)),
    ] {
        spawn_box(&mut commands, &mut meshes, &obstacle, position, size, Some(size.xz() * 0.5));
    }

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(BALL_RADIUS).mesh().uv(32, 20))),
        MeshMaterial3d(dung),
        Transform::from_xyz(0.0, BALL_RADIUS, 6.5),
        Ball { velocity: Vec3::ZERO },
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.12, 0.08))),
        MeshMaterial3d(nest),
        Transform::from_xyz(0.0, 0.02, -7.5),
        Nest,
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9_000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.5, 0.0)),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.05, 9.5),
        Beetle,
        LookAngles { yaw: 0.0, pitch: -0.05 },
    ));
}

fn spawn_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    obstacle: Option<Vec2>,
) {
    let mut entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(position),
    ));
    if let Some(half_extent) = obstacle {
        entity.insert(Obstacle { half_extent });
    }
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Node { position_type: PositionType::Absolute, top: px(18), left: px(20), ..default() },
        Text::new(""),
        TextFont { font_size: FontSize::Px(22.0), ..default() },
        TextColor(Color::srgb(1.0, 0.88, 0.62)),
        Visibility::Hidden,
        HudText,
    ));
}

fn open_title_screen(mut screens: ResMut<GutzUiStack<UiScreen>>) {
    screens.push(UiScreen::Title);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn start_from_menu(
    actions: Res<GutzActionState<BeetleAction>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
    mut hud: Query<&mut Visibility, With<HudText>>,
    mut progress: ResMut<PuzzleProgress>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<&mut Transform, (With<Nest>, Without<Ball>, Without<Beetle>)>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
) {
    if !actions.just_pressed(BeetleAction::Start) {
        return;
    }

    progress.stage = 0;
    place_stage(&progress, &mut ball, &mut nest, &mut beetle);
    screens.clear();
    next_state.set(GameState::Playing);
    *hud.single_mut().expect("HUD is spawned during startup") = Visibility::Visible;

    if *state.get() == GameState::Cleared {
        // クリア画面からの開始も同じ初期化経路へ集約する。
        progress.best_stage = progress.best_stage.max(STAGE_COUNT);
    }
}

#[allow(clippy::type_complexity)]
fn update_player(
    time: Res<Time>,
    actions: Res<GutzActionState<BeetleAction>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Obstacle>)>,
    obstacles: Query<(&Transform, &Obstacle), Without<Beetle>>,
) {
    let (transform, look) = &mut *beetle;
    let mouse_delta = mouse_motion.read().map(|event| event.delta).sum::<Vec2>();
    look.yaw -= mouse_delta.x * 0.0025;
    look.pitch = (look.pitch - mouse_delta.y * 0.0025).clamp(-0.72, 0.55);

    let turn = f32::from(actions.pressed(BeetleAction::TurnRight))
        - f32::from(actions.pressed(BeetleAction::TurnLeft));
    look.yaw -= turn * time.delta_secs() * 1.9;
    transform.rotation = Quat::from_rotation_y(look.yaw) * Quat::from_rotation_x(look.pitch);

    let forward = Quat::from_rotation_y(look.yaw) * Vec3::NEG_Z;
    let right = Quat::from_rotation_y(look.yaw) * Vec3::X;
    let movement = forward
        * (f32::from(actions.pressed(BeetleAction::Forward))
            - f32::from(actions.pressed(BeetleAction::Backward)))
        + right
            * (f32::from(actions.pressed(BeetleAction::Right))
                - f32::from(actions.pressed(BeetleAction::Left)));
    if movement.length_squared() > 0.0 {
        transform.translation += movement.normalize() * time.delta_secs() * 4.2;
        transform.translation.x =
            transform.translation.x.clamp(-ARENA_HALF_SIZE + 0.8, ARENA_HALF_SIZE - 0.8);
        transform.translation.z =
            transform.translation.z.clamp(-ARENA_HALF_SIZE + 0.8, ARENA_HALF_SIZE - 0.8);
        for (obstacle_transform, obstacle) in &obstacles {
            resolve_beetle_obstacle(
                transform,
                obstacle_transform.translation,
                obstacle.half_extent,
            );
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn roll_ball(
    time: Res<Time>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<&mut Transform, (With<Nest>, Without<Ball>, Without<Beetle>)>,
    obstacles: Query<(&Transform, &Obstacle), (Without<Ball>, Without<Beetle>, Without<Nest>)>,
    mut progress: ResMut<PuzzleProgress>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
    mut hud: Query<&mut Visibility, With<HudText>>,
    mut saves: MessageWriter<GutzSaveRequest<SaveData>>,
) {
    let (ball_transform, ball_state) = &mut *ball;
    let to_ball = ball_transform.translation - beetle.0.translation;
    let planar = Vec2::new(to_ball.x, to_ball.z);
    let contact_distance = BALL_RADIUS + BEETLE_RADIUS;
    if planar.length_squared() < contact_distance * contact_distance {
        let push_direction = planar.normalize_or_zero();
        ball_state.velocity.x += push_direction.x * time.delta_secs() * 19.0;
        ball_state.velocity.z += push_direction.y * time.delta_secs() * 19.0;
    }

    ball_transform.translation += ball_state.velocity * time.delta_secs();
    ball_transform.translation.y = BALL_RADIUS;
    ball_state.velocity *= 0.985_f32.powf(time.delta_secs() * 60.0);
    resolve_arena_bounds(ball_transform, ball_state);
    for (obstacle_transform, obstacle) in &obstacles {
        resolve_obstacle(
            ball_transform,
            ball_state,
            obstacle_transform.translation,
            obstacle.half_extent,
        );
    }
    ball_transform.rotate_local_x(-ball_state.velocity.length() * time.delta_secs() / BALL_RADIUS);

    let distance_to_nest = ball_transform.translation.xz().distance(nest.translation.xz());
    if distance_to_nest > 0.86 {
        return;
    }

    progress.stage += 1;
    progress.best_stage = progress.best_stage.max(progress.stage);
    saves.write(GutzSaveRequest(SaveData { best_stage: progress.best_stage }));
    if progress.stage < STAGE_COUNT {
        let (ball_position, nest_position, beetle_position, yaw) = stage_layout(progress.stage);
        ball_transform.translation = ball_position;
        ball_transform.translation.y = BALL_RADIUS;
        ball_state.velocity = Vec3::ZERO;
        nest.translation = nest_position;
        beetle.0.translation = beetle_position;
        beetle.1.yaw = yaw;
        beetle.1.pitch = -0.05;
        beetle.0.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(beetle.1.pitch);
    } else {
        next_state.set(GameState::Cleared);
        screens.push(UiScreen::Cleared);
        *hud.single_mut().expect("HUD is spawned during startup") = Visibility::Hidden;
    }
}

#[allow(clippy::type_complexity)]
fn reset_stage(
    actions: Res<GutzActionState<BeetleAction>>,
    progress: Res<PuzzleProgress>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<&mut Transform, (With<Nest>, Without<Ball>, Without<Beetle>)>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
) {
    if actions.just_pressed(BeetleAction::ResetStage) {
        place_stage(&progress, &mut ball, &mut nest, &mut beetle);
    }
}

fn place_stage(
    progress: &PuzzleProgress,
    ball: &mut (Mut<Transform>, Mut<Ball>),
    nest: &mut Mut<Transform>,
    beetle: &mut (Mut<Transform>, Mut<LookAngles>),
) {
    let (ball_position, nest_position, beetle_position, yaw) = stage_layout(progress.stage);
    ball.0.translation = ball_position;
    ball.0.translation.y = BALL_RADIUS;
    ball.1.velocity = Vec3::ZERO;
    nest.translation = nest_position;
    beetle.0.translation = beetle_position;
    beetle.1.yaw = yaw;
    beetle.1.pitch = -0.05;
    beetle.0.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(beetle.1.pitch);
}

fn stage_layout(stage: u8) -> (Vec3, Vec3, Vec3, f32) {
    match stage % STAGE_COUNT {
        0 => (
            Vec3::new(0.0, BALL_RADIUS, 6.5),
            Vec3::new(0.0, 0.02, -7.5),
            Vec3::new(0.0, 1.05, 9.5),
            0.0,
        ),
        1 => (
            Vec3::new(-7.8, BALL_RADIUS, 5.8),
            Vec3::new(7.6, 0.02, -6.6),
            Vec3::new(-7.8, 1.05, 8.5),
            -0.35,
        ),
        _ => (
            Vec3::new(7.4, BALL_RADIUS, 7.2),
            Vec3::new(-7.7, 0.02, -6.8),
            Vec3::new(8.4, 1.05, 9.3),
            0.4,
        ),
    }
}

fn resolve_arena_bounds(transform: &mut Transform, ball: &mut Ball) {
    let edge = ARENA_HALF_SIZE - BALL_RADIUS - 0.25;
    if transform.translation.x.abs() > edge {
        transform.translation.x = transform.translation.x.clamp(-edge, edge);
        ball.velocity.x *= -0.45;
    }
    if transform.translation.z.abs() > edge {
        transform.translation.z = transform.translation.z.clamp(-edge, edge);
        ball.velocity.z *= -0.45;
    }
}

fn resolve_beetle_obstacle(transform: &mut Transform, center: Vec3, half_extent: Vec2) {
    let point = transform.translation.xz();
    let closest = point.clamp(center.xz() - half_extent, center.xz() + half_extent);
    let offset = point - closest;
    if offset.length_squared() >= BEETLE_RADIUS * BEETLE_RADIUS {
        return;
    }
    let normal = if offset.length_squared() > f32::EPSILON { offset.normalize() } else { Vec2::X };
    let corrected = closest + normal * BEETLE_RADIUS;
    transform.translation.x = corrected.x;
    transform.translation.z = corrected.y;
}

fn resolve_obstacle(transform: &mut Transform, ball: &mut Ball, center: Vec3, half_extent: Vec2) {
    let point = transform.translation.xz();
    let closest = point.clamp(center.xz() - half_extent, center.xz() + half_extent);
    let offset = point - closest;
    if offset.length_squared() >= BALL_RADIUS * BALL_RADIUS {
        return;
    }
    let normal = if offset.length_squared() > f32::EPSILON {
        offset.normalize()
    } else if (point.x - center.x).abs() > (point.y - center.z).abs() {
        Vec2::new((point.x - center.x).signum(), 0.0)
    } else {
        Vec2::new(0.0, (point.y - center.z).signum())
    };
    let corrected = closest + normal * BALL_RADIUS;
    transform.translation.x = corrected.x;
    transform.translation.z = corrected.y;
    let planar_velocity = Vec2::new(ball.velocity.x, ball.velocity.z);
    let reflected = planar_velocity - normal * (2.0 * planar_velocity.dot(normal));
    ball.velocity.x = reflected.x * 0.5;
    ball.velocity.z = reflected.y * 0.5;
}

fn update_hud(progress: Res<PuzzleProgress>, mut text: Query<&mut Text, With<HudText>>) {
    **text.single_mut().expect("HUD is spawned during startup") = format!(
        "フンコロ  |  巣穴 {}/{}\nWASD: 歩く  マウス / Q E: 見回す  R: やり直し",
        progress.stage + 1,
        STAGE_COUNT
    );
}

fn open_or_close_screen(
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
            UiScreen::Title => ("フンコロ", "玉を巣穴まで押し運べ。\n\nEnter / Space で地中へ"),
            UiScreen::Cleared => ("巣穴完成", "三つの玉を運び終えた。\n\nEnter / Space でもう一度"),
        };
        spawn_modal_panel(&mut commands, GutzModalPanelStyle::default())
            .insert(ScreenUi)
            .with_children(|panel| {
                panel.spawn((
                    Text::new(title),
                    TextFont { font_size: FontSize::Px(56.0), ..default() },
                    TextColor(Color::srgb(1.0, 0.82, 0.38)),
                ));
                panel.spawn((
                    Text::new(body),
                    TextFont { font_size: FontSize::Px(25.0), ..default() },
                    TextColor(Color::srgb(1.0, 0.93, 0.78)),
                ));
            });
    }
}
