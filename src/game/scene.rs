use bevy::prelude::*;

use super::{ARENA_HALF_SIZE, START_BALL_RADIUS};

#[derive(Component)]
pub(super) struct Beetle;

#[derive(Component)]
pub(super) struct Ball {
    pub(super) radius: f32,
    pub(super) traveled_distance: f32,
}

#[derive(Component)]
pub(super) struct Nest {
    pub(super) hole_radius: f32,
}

#[derive(Component)]
pub(super) struct Obstacle {
    pub(super) half_extent: Vec2,
}

#[derive(Component)]
pub(super) struct MudPatch {
    pub(super) radius: f32,
    pub(super) growth_per_meter: f32,
}

#[derive(Component)]
pub(super) struct LookAngles {
    pub(super) yaw: f32,
    pub(super) pitch: f32,
}

pub(super) fn setup_world(
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
    let hole_dark = materials.add(Color::srgb(0.004, 0.001, 0.0));
    let hole_rim = materials.add(Color::srgb(0.16, 0.065, 0.016));
    let mud = materials.add(Color::srgb(0.08, 0.032, 0.008));
    let limb = materials.add(StandardMaterial {
        base_color: Color::srgb(0.045, 0.018, 0.006),
        perceptual_roughness: 0.96,
        ..default()
    });

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
    for (position, radius, growth_per_meter) in [
        (Vec3::new(-6.2, 0.025, 3.4), 2.3, 0.075),
        (Vec3::new(4.8, 0.025, 0.3), 2.7, 0.1),
        (Vec3::new(-1.1, 0.025, -6.0), 2.1, 0.13),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(radius, 0.04))),
            MeshMaterial3d(mud.clone()),
            Transform::from_translation(position),
            MudPatch { radius, growth_per_meter },
        ));
    }

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(START_BALL_RADIUS).mesh().uv(32, 20))),
        MeshMaterial3d(dung),
        Transform::from_xyz(0.0, START_BALL_RADIUS, -6.5),
        Ball { radius: START_BALL_RADIUS, traveled_distance: 0.0 },
    ));
    commands
        .spawn((Transform::from_xyz(0.0, 0.02, 7.5), Nest { hole_radius: 0.74 }))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.84, 0.22))),
                MeshMaterial3d(hole_dark),
                Transform::from_xyz(0.0, -0.1, 0.0),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Torus::new(0.84, 1.0))),
                MeshMaterial3d(hole_rim),
                Transform::from_xyz(0.0, 0.035, 0.0),
            ));
        });
    commands.spawn((
        DirectionalLight { illuminance: 9_000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.5, 0.0)),
    ));
    let beetle = commands
        .spawn((
            Camera3d::default(),
            Projection::from(PerspectiveProjection { fov: 1.35, ..default() }),
            Transform::from_xyz(0.0, 0.48, -9.5).with_rotation(Quat::from_rotation_x(-0.62)),
            Beetle,
            LookAngles { yaw: 0.0, pitch: -0.62 },
        ))
        .id();
    spawn_view_legs(&mut commands, &mut meshes, limb, beetle);
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

fn spawn_view_legs(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    beetle: Entity,
) {
    let limb_mesh = meshes.add(Cuboid::new(0.16, 0.13, 0.9));
    let claw_mesh = meshes.add(Cuboid::new(0.22, 0.1, 0.24));
    commands.entity(beetle).with_children(|parent| {
        for (x, roll) in [(-0.42, -0.38), (0.42, 0.38)] {
            parent.spawn((
                Mesh3d(limb_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(x, -0.27, -0.62)
                    .with_rotation(Quat::from_rotation_z(roll) * Quat::from_rotation_x(-0.48)),
            ));
            parent.spawn((
                Mesh3d(claw_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(x * 1.18, -0.36, -1.0)
                    .with_rotation(Quat::from_rotation_z(roll * 0.5)),
            ));
        }
    });
}
