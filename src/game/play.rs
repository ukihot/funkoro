use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_gutzgutz::{input::GutzActionState, save::GutzSaveRequest, ui::GutzUiStack};

use super::{
    ARENA_HALF_SIZE, BEETLE_RADIUS, BeetleAction, GameState, MAX_BALL_RADIUS, PuzzleProgress,
    STAGE_COUNT, START_BALL_RADIUS, SaveData, UiScreen,
    scene::{Ball, Beetle, LookAngles, MudPatch, Nest, Obstacle},
};

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update_beetle(
    time: Res<Time>,
    actions: Res<GutzActionState<BeetleAction>>,
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
    let forced_pitch = (-0.58_f32).lerp(-1.43, growth);
    let mouse_delta = mouse_motion.read().map(|event| event.delta).sum::<Vec2>();
    look.yaw -= mouse_delta.x * 0.0025;
    look.pitch =
        (look.pitch - mouse_delta.y * 0.0025).clamp(forced_pitch - 0.12, forced_pitch + 0.08);
    if let Projection::Perspective(perspective) = &mut **projection {
        perspective.fov = 1.35_f32.lerp(0.72, growth);
    }

    let turn = f32::from(actions.pressed(BeetleAction::TurnRight))
        - f32::from(actions.pressed(BeetleAction::TurnLeft));
    look.yaw -= turn * time.delta_secs() * 1.9;
    transform.rotation = Quat::from_rotation_y(look.yaw) * Quat::from_rotation_x(look.pitch);

    // `S` はカメラ基準で明確に後退（+Z）する。玉は背後に置かれ、この後退が
    // フンコロガシの後脚で玉を巣穴方向へ押し込む唯一の移動操作になる。
    let movement = Quat::from_rotation_y(look.yaw)
        * Vec3::Z
        * f32::from(actions.pressed(BeetleAction::PushBackward));
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
    resolve_beetle_ball(transform, ball_transform, ball_state, attempted_step);
}

pub(super) fn open_menu(
    actions: Res<GutzActionState<BeetleAction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
) {
    if actions.just_pressed(BeetleAction::OpenMenu) {
        next_state.set(GameState::Menu);
        screens.push(UiScreen::Menu);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn roll_ball(
    time: Res<Time>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<(&mut Transform, &mut Nest), (Without<Ball>, Without<Beetle>)>,
    obstacles: Query<(&Transform, &Obstacle), (Without<Ball>, Without<Beetle>, Without<Nest>)>,
    mud: Query<(&Transform, &MudPatch), (Without<Ball>, Without<Beetle>, Without<Nest>)>,
    mut progress: ResMut<PuzzleProgress>,
    mut next_state: ResMut<NextState<GameState>>,
    mut screens: ResMut<GutzUiStack<UiScreen>>,
    mut saves: MessageWriter<GutzSaveRequest<SaveData>>,
) {
    let (ball_transform, ball_state) = &mut *ball;
    let distance = ball_state.velocity.xz().length() * time.delta_secs();
    ball_transform.translation += ball_state.velocity * time.delta_secs();
    ball_state.velocity *= 0.985_f32.powf(time.delta_secs() * 60.0);

    grow_in_mud(ball_transform, ball_state, distance, &mud);
    ball_transform.translation.y = ball_state.radius;
    resolve_arena_bounds(ball_transform, ball_state);
    for (obstacle_transform, obstacle) in &obstacles {
        resolve_obstacle(
            ball_transform,
            ball_state,
            obstacle_transform.translation,
            obstacle.half_extent,
        );
    }
    resolve_ball_against_beetle(ball_transform, ball_state, beetle.0.translation);
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
        place_stage(&progress, &mut ball, &mut nest, &mut beetle);
    } else {
        next_state.set(GameState::Cleared);
        screens.push(UiScreen::Cleared);
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn reset_stage(
    actions: Res<GutzActionState<BeetleAction>>,
    progress: Res<PuzzleProgress>,
    mut ball: Single<(&mut Transform, &mut Ball), Without<Beetle>>,
    mut nest: Single<(&mut Transform, &mut Nest), (Without<Ball>, Without<Beetle>)>,
    mut beetle: Single<(&mut Transform, &mut LookAngles), (With<Beetle>, Without<Ball>)>,
) {
    if actions.just_pressed(BeetleAction::ResetStage) {
        place_stage(&progress, &mut ball, &mut nest, &mut beetle);
    }
}

pub(super) fn place_stage(
    progress: &PuzzleProgress,
    ball: &mut (Mut<Transform>, Mut<Ball>),
    nest: &mut (Mut<Transform>, Mut<Nest>),
    beetle: &mut (Mut<Transform>, Mut<LookAngles>),
) {
    let (ball_position, nest_position, beetle_position, yaw, hole_radius) =
        stage_layout(progress.stage);
    ball.0.translation = ball_position;
    ball.0.scale = Vec3::ONE;
    ball.1.radius = START_BALL_RADIUS;
    ball.1.velocity = Vec3::ZERO;
    nest.0.translation = nest_position;
    nest.0.scale = Vec3::splat(hole_radius);
    nest.1.hole_radius = hole_radius;
    beetle.0.translation = beetle_position;
    beetle.1.yaw = yaw;
    beetle.1.pitch = -0.58;
    beetle.0.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(beetle.1.pitch);
}

fn stage_layout(stage: u8) -> (Vec3, Vec3, Vec3, f32, f32) {
    match stage % STAGE_COUNT {
        0 => (
            Vec3::new(0.0, START_BALL_RADIUS, -6.5),
            Vec3::new(0.0, 0.02, 7.5),
            Vec3::new(0.0, 0.48, -9.5),
            0.0,
            0.74,
        ),
        1 => (
            Vec3::new(-7.8, START_BALL_RADIUS, -5.8),
            Vec3::new(7.6, 0.02, 6.6),
            Vec3::new(-7.8, 0.48, -8.5),
            -0.35,
            0.92,
        ),
        _ => (
            Vec3::new(7.4, START_BALL_RADIUS, -7.2),
            Vec3::new(-7.7, 0.02, 6.8),
            Vec3::new(8.4, 0.48, -9.3),
            0.4,
            1.1,
        ),
    }
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

fn resolve_arena_bounds(transform: &mut Transform, ball: &mut Ball) {
    let edge = ARENA_HALF_SIZE - ball.radius - 0.25;
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

fn resolve_beetle_ball(
    beetle: &mut Transform,
    ball_transform: &mut Transform,
    ball: &mut Ball,
    step: Vec3,
) {
    let offset = beetle.translation.xz() - ball_transform.translation.xz();
    let distance = BEETLE_RADIUS + ball.radius;
    if offset.length_squared() >= distance * distance {
        return;
    }
    let normal = offset.normalize_or_zero();
    let normal = if normal == Vec2::ZERO { -step.xz().normalize_or_zero() } else { normal };
    let normal = if normal == Vec2::ZERO { Vec2::Y } else { normal };
    let corrected = ball_transform.translation.xz() + normal * distance;
    beetle.translation.x = corrected.x;
    beetle.translation.z = corrected.y;
    let push = -normal * (-normal).dot(step.xz()).max(0.0) * 14.0;
    ball.velocity.x += push.x;
    ball.velocity.z += push.y;
}

fn resolve_ball_against_beetle(ball_transform: &mut Transform, ball: &mut Ball, beetle: Vec3) {
    let offset = ball_transform.translation.xz() - beetle.xz();
    let distance = BEETLE_RADIUS + ball.radius;
    if offset.length_squared() >= distance * distance {
        return;
    }
    let normal = offset.normalize_or_zero();
    let normal = if normal == Vec2::ZERO { Vec2::NEG_Y } else { normal };
    let corrected = beetle.xz() + normal * distance;
    ball_transform.translation.x = corrected.x;
    ball_transform.translation.z = corrected.y;
    let velocity = Vec2::new(ball.velocity.x, ball.velocity.z);
    let reflected = velocity - normal * (2.0 * velocity.dot(normal));
    ball.velocity.x = reflected.x * 0.35;
    ball.velocity.z = reflected.y * 0.35;
}

fn resolve_obstacle(transform: &mut Transform, ball: &mut Ball, center: Vec3, half_extent: Vec2) {
    let point = transform.translation.xz();
    let closest = point.clamp(center.xz() - half_extent, center.xz() + half_extent);
    let offset = point - closest;
    if offset.length_squared() >= ball.radius * ball.radius {
        return;
    }
    let normal = if offset.length_squared() > f32::EPSILON {
        offset.normalize()
    } else if (point.x - center.x).abs() > (point.y - center.z).abs() {
        Vec2::new((point.x - center.x).signum(), 0.0)
    } else {
        Vec2::new(0.0, (point.y - center.z).signum())
    };
    let corrected = closest + normal * ball.radius;
    transform.translation.x = corrected.x;
    transform.translation.z = corrected.y;
    let velocity = Vec2::new(ball.velocity.x, ball.velocity.z);
    let reflected = velocity - normal * (2.0 * velocity.dot(normal));
    ball.velocity.x = reflected.x * 0.5;
    ball.velocity.z = reflected.y * 0.5;
}
