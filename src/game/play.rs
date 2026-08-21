use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_gutzgutz::{input::ActionState, save::GutzSaveRequest, ui::GutzUiStack};

use super::{
    ARENA_HALF_SIZE, BEETLE_RADIUS, BeetleAction, ClearScore, GameState, HoleLayout,
    MAX_BALL_RADIUS, PuzzleProgress, RunLayout, STAGE_COUNT, START_BALL_RADIUS, SaveData, UiScreen,
    scene::{Ball, Beetle, LookAngles, MudPatch, Nest, Obstacle},
};

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update_beetle(
    time: Res<Time>,
    actions: Single<&ActionState<BeetleAction>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut beetle: Single<
        (&mut Transform, &mut LookAngles, &mut Projection),
        (With<Beetle>, Without<Obstacle>, Without<Ball>),
    >,
    mut ball: Single<(&mut Transform, &mut Ball), (Without<Beetle>, Without<Obstacle>)>,
    obstacles: Query<(&Transform, &Obstacle), (Without<Beetle>, Without<Ball>)>,
) {
    let (transform, look, projection) = &mut *beetle;
    let (_, ball_state) = &*ball;
    let growth = growth_fraction(ball_state.radius);
    // 成長の大半では視界を保ち、限界サイズが近づいた時だけ地面へ押し付ける。
    let visual_growth = view_pressure_from_growth(growth);
    let forced_pitch = (-0.58_f32).lerp(-1.43, visual_growth);
    let mouse_delta = mouse_motion.read().map(|event| event.delta).sum::<Vec2>();
    look.yaw -= mouse_delta.x * 0.0025;
    look.pitch =
        (look.pitch - mouse_delta.y * 0.0025).clamp(forced_pitch - 0.12, forced_pitch + 0.08);
    if let Projection::Perspective(perspective) = &mut **projection {
        perspective.fov = 1.35_f32.lerp(0.72, visual_growth);
    }

    let turn = f32::from(actions.pressed(&BeetleAction::TurnRight))
        - f32::from(actions.pressed(&BeetleAction::TurnLeft));
    look.yaw -= turn * time.delta_secs() * 1.9;
    transform.rotation = Quat::from_rotation_y(look.yaw) * Quat::from_rotation_x(look.pitch);

    // `S` はカメラ基準で明確に後退（+Z）する。玉は背後に置かれ、この後退が
    // フンコロガシの後脚で玉を巣穴方向へ押し込む唯一の移動操作になる。
    let movement = Quat::from_rotation_y(look.yaw)
        * Vec3::Z
        * f32::from(actions.pressed(&BeetleAction::PushBackward));
    if movement.length_squared() == 0.0 {
        return;
    }

    let attempted_step = movement.normalize() * time.delta_secs() * 3.6;
    transform.translation += attempted_step;
    transform.translation.x =
        transform.translation.x.clamp(-ARENA_HALF_SIZE + 0.8, ARENA_HALF_SIZE - 0.8);
    transform.translation.z =
        transform.translation.z.clamp(-ARENA_HALF_SIZE + 0.8, ARENA_HALF_SIZE - 0.8);
    for (obstacle_transform, obstacle) in &obstacles {
        resolve_beetle_obstacle(transform, obstacle_transform.translation, obstacle.half_extent);
    }
    let (ball_transform, ball_state) = &mut *ball;
    ball_state.traveled_distance += attempted_step.length();
    constrain_carrier(transform, ball_transform, ball_state.radius, look.yaw, &obstacles);
}

pub(super) fn open_menu(
    actions: Single<&ActionState<BeetleAction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
) {
    if actions.just_pressed(&BeetleAction::OpenMenu) {
        next_state.set(GameState::Menu);
        screens.push(UiScreen::Menu);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn roll_ball(
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<(&mut Transform, &mut Nest), (Without<Ball>, Without<Beetle>)>,
    mud: Query<(&Transform, &MudPatch), (Without<Ball>, Without<Beetle>, Without<Nest>)>,
    mut progress: ResMut<PuzzleProgress>,
    mut run_layout: ResMut<RunLayout>,
    mut clear_score: ResMut<ClearScore>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
    mut saves: MessageWriter<GutzSaveRequest<SaveData>>,
) {
    let (ball_transform, ball_state) = &mut *ball;
    let distance = core::mem::take(&mut ball_state.traveled_distance);

    grow_in_mud(ball_transform, ball_state, distance, &mud);
    attach_ball(ball_transform, beetle.0.translation, beetle.1.yaw, ball_state.radius);
    ball_transform.rotate_local_x(-distance / ball_state.radius.max(0.01));

    let distance_to_nest = ball_transform.translation.xz().distance(nest.0.translation.xz());
    if distance_to_nest > nest.1.hole_radius + ball_state.radius {
        return;
    }

    let lower = nest.1.hole_radius * 0.92;
    let upper = nest.1.hole_radius * 1.16;
    if !(lower..=upper).contains(&ball_state.radius) {
        next_state.set(GameState::Stuck);
        screens.push(UiScreen::Stuck);
        return;
    }

    progress.stage += 1;
    progress.best_stage = progress.best_stage.max(progress.stage);
    saves.write(GutzSaveRequest(SaveData { best_stage: progress.best_stage }));
    if progress.stage < STAGE_COUNT {
        place_stage(&progress, &mut run_layout, true, &mut ball, &mut nest, &mut beetle);
    } else {
        if clear_score.first_clear_diameter_cm.is_none() {
            clear_score.first_clear_diameter_cm = Some((ball_state.radius * 200.0).round() as u32);
        }
        next_state.set(GameState::Cleared);
        screens.push(UiScreen::Cleared);
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn reset_stage(
    actions: Single<&ActionState<BeetleAction>>,
    progress: Res<PuzzleProgress>,
    mut run_layout: ResMut<RunLayout>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<(&mut Transform, &mut Nest), (Without<Ball>, Without<Beetle>)>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
) {
    if actions.just_pressed(&BeetleAction::ResetStage) {
        place_stage(&progress, &mut run_layout, false, &mut ball, &mut nest, &mut beetle);
    }
}

pub(super) fn place_stage(
    progress: &PuzzleProgress,
    run_layout: &mut RunLayout,
    reroll_hole: bool,
    ball: &mut (Mut<Transform>, Mut<Ball>),
    nest: &mut (Mut<Transform>, Mut<Nest>),
    beetle: &mut (Mut<Transform>, Mut<LookAngles>),
) {
    let (ball_position, beetle_position, yaw) = stage_layout(progress.stage);
    if reroll_hole || run_layout.hole.is_none_or(|hole| hole.stage != progress.stage) {
        run_layout.hole = Some(generate_hole(progress.stage, ball_position, run_layout));
    }
    let hole = run_layout.hole.expect("stage placement always creates a hole");
    ball.0.translation = beetle_position
        + Quat::from_rotation_y(yaw) * Vec3::Z * (BEETLE_RADIUS + START_BALL_RADIUS);
    ball.0.scale = Vec3::ONE;
    ball.1.radius = START_BALL_RADIUS;
    ball.1.traveled_distance = 0.0;
    nest.0.translation = hole.position;
    nest.0.scale = Vec3::splat(hole.radius);
    nest.1.hole_radius = hole.radius;
    beetle.0.translation = beetle_position;
    beetle.1.yaw = yaw;
    beetle.1.pitch = -0.58;
    beetle.0.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(beetle.1.pitch);
}

fn stage_layout(stage: u8) -> (Vec3, Vec3, f32) {
    match stage % STAGE_COUNT {
        0 => (Vec3::new(0.0, START_BALL_RADIUS, -6.5), Vec3::new(0.0, 0.48, -9.5), 0.0),
        1 => (Vec3::new(-7.8, START_BALL_RADIUS, -5.8), Vec3::new(-7.8, 0.48, -8.5), -0.35),
        _ => (Vec3::new(7.4, START_BALL_RADIUS, -7.2), Vec3::new(8.4, 0.48, -9.3), 0.4),
    }
}

fn generate_hole(stage: u8, ball_position: Vec3, run_layout: &mut RunLayout) -> HoleLayout {
    let mut position = Vec3::ZERO;
    for _ in 0..12 {
        let angle = next_random(run_layout) * core::f32::consts::TAU;
        let distance = 7.2 + next_random(run_layout) * 2.7;
        position = Vec3::new(angle.cos() * distance, 0.02, angle.sin() * distance);
        if position.xz().distance(ball_position.xz()) > 10.0 {
            break;
        }
    }
    let direct_distance = position.xz().distance(ball_position.xz());
    let expected_course = direct_distance * (1.0 + f32::from(stage) * 0.28);
    let radius = (0.52 + expected_course * 0.035).clamp(0.68, 1.22);
    HoleLayout { stage, position, radius }
}

fn next_random(run_layout: &mut RunLayout) -> f32 {
    run_layout.random_state = run_layout
        .random_state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let bits = (run_layout.random_state >> 40) as u32;
    bits as f32 / ((1_u32 << 24) as f32)
}

#[allow(clippy::type_complexity)]
fn grow_in_mud(
    ball_transform: &mut Transform,
    ball: &mut Ball,
    distance: f32,
    mud: &Query<(&Transform, &MudPatch), (Without<Ball>, Without<Beetle>, Without<Nest>)>,
) {
    for (patch_transform, patch) in mud {
        if ball_transform.translation.xz().distance(patch_transform.translation.xz())
            <= patch.radius + ball.radius
        {
            ball.radius = (ball.radius + distance * patch.growth_per_meter).min(MAX_BALL_RADIUS);
            ball_transform.scale = Vec3::splat(ball.radius / START_BALL_RADIUS);
        }
    }
}

fn growth_fraction(radius: f32) -> f32 {
    ((radius - START_BALL_RADIUS) / (MAX_BALL_RADIUS - START_BALL_RADIUS)).clamp(0.0, 1.0)
}

pub(super) fn view_pressure(radius: f32) -> f32 {
    view_pressure_from_growth(growth_fraction(radius))
}

fn view_pressure_from_growth(growth: f32) -> f32 {
    growth.powi(6)
}

#[allow(clippy::type_complexity)]
fn constrain_carrier(
    beetle: &mut Transform,
    ball: &mut Transform,
    ball_radius: f32,
    yaw: f32,
    obstacles: &Query<(&Transform, &Obstacle), (Without<Beetle>, Without<Ball>)>,
) {
    for _ in 0..2 {
        attach_ball(ball, beetle.translation, yaw, ball_radius);
        constrain_ball_bounds(ball, ball_radius);
        for (obstacle_transform, obstacle) in obstacles {
            resolve_ball_obstacle(
                ball,
                ball_radius,
                obstacle_transform.translation,
                obstacle.half_extent,
            );
        }
        beetle.translation =
            ball.translation - Quat::from_rotation_y(yaw) * Vec3::Z * (BEETLE_RADIUS + ball_radius);
        beetle.translation.x =
            beetle.translation.x.clamp(-ARENA_HALF_SIZE + 0.8, ARENA_HALF_SIZE - 0.8);
        beetle.translation.z =
            beetle.translation.z.clamp(-ARENA_HALF_SIZE + 0.8, ARENA_HALF_SIZE - 0.8);
        for (obstacle_transform, obstacle) in obstacles {
            resolve_beetle_obstacle(beetle, obstacle_transform.translation, obstacle.half_extent);
        }
    }
}

fn attach_ball(ball: &mut Transform, beetle_position: Vec3, yaw: f32, radius: f32) {
    ball.translation =
        beetle_position + Quat::from_rotation_y(yaw) * Vec3::Z * (BEETLE_RADIUS + radius);
    ball.translation.y = radius;
}

fn constrain_ball_bounds(transform: &mut Transform, radius: f32) {
    let edge = ARENA_HALF_SIZE - radius - 0.25;
    if transform.translation.x.abs() > edge {
        transform.translation.x = transform.translation.x.clamp(-edge, edge);
    }
    if transform.translation.z.abs() > edge {
        transform.translation.z = transform.translation.z.clamp(-edge, edge);
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

fn resolve_ball_obstacle(transform: &mut Transform, radius: f32, center: Vec3, half_extent: Vec2) {
    let point = transform.translation.xz();
    let closest = point.clamp(center.xz() - half_extent, center.xz() + half_extent);
    let offset = point - closest;
    if offset.length_squared() >= radius * radius {
        return;
    }
    let normal = if offset.length_squared() > f32::EPSILON {
        offset.normalize()
    } else if (point.x - center.x).abs() > (point.y - center.z).abs() {
        Vec2::new((point.x - center.x).signum(), 0.0)
    } else {
        Vec2::new(0.0, (point.y - center.z).signum())
    };
    let corrected = closest + normal * radius;
    transform.translation.x = corrected.x;
    transform.translation.z = corrected.y;
}
