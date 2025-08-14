//! # Rolling Circles
//! This scene includes two circles rolling around each other.
//! This is a simple demonstration of a physics model I was working on.
//! The angular velocities and circle radii are hard-coded, calculated with a numerical equations solver.
//! I actually got two sets of solutions, but only one is used here.
//! This program is added the `PanCamPlugin`, so users can zoom or drag the camera around.

use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use creative_bevy::plugins::esc_exit_plugin::EscExitPlugin;

#[derive(Component)]
struct AngularVelocity(f32);

#[derive(Component)]
struct OrbitAngularVelocity(f32);

#[derive(Component)]
struct Distance(f32);

/// Information for spawning a circle.
struct CircleInfo {
    radius: f32,
    x: f32,
    color: Color,
    line_color: Handle<ColorMaterial>,
    angular_velocity: AngularVelocity,
    orbit_angular_velocity: OrbitAngularVelocity,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((DefaultPlugins, PanCamPlugin, EscExitPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate_bodies, move_bodies))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Camera
    commands.spawn((Camera2d, PanCam::default()));

    // The origin circle
    commands.spawn((
        Mesh2d(meshes.add(Mesh::from(Circle::new(0.3)))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));

    let line_color = materials.add(Color::WHITE);

    let m1 = 2.0;
    let m2 = 1.0;
    let r1 = 10.0;
    let r2 = 5.0;
    let orbit_ang_vel = 0.512097661192167;

    let d1 = m2 * (r1 + r2) / (m1 + m2);
    let d2 = m1 * (r1 + r2) / (m1 + m2);

    // circle 1
    spawn_circle(
        &mut commands,
        &mut meshes,
        &mut materials,
        CircleInfo {
            radius: r1,
            x: -d1, // negative x
            color: Color::linear_rgb(1.0, 0.0, 0.0),
            line_color: line_color.clone(),
            angular_velocity: AngularVelocity(0.304439475364754),
            orbit_angular_velocity: OrbitAngularVelocity(orbit_ang_vel),
        },
    );

    // circle 2
    spawn_circle(
        &mut commands,
        &mut meshes,
        &mut materials,
        CircleInfo {
            radius: r2,
            x: d2,
            color: Color::linear_rgb(0.0, 1.0, 0.0),
            line_color,
            angular_velocity: AngularVelocity(0.927414032846995),
            orbit_angular_velocity: OrbitAngularVelocity(orbit_ang_vel),
        },
    );
}

fn rotate_bodies(
    time: Res<Time>,
    mut query: Query<(&AngularVelocity, &mut Transform), With<Mesh2d>>,
) {
    for (angular_velocity, mut transform) in query.iter_mut() {
        let translation = transform.translation;

        *transform = Transform::from_rotation(Quat::from_rotation_z(
            angular_velocity.0 * time.elapsed_secs(),
        ));

        transform.translation = translation;
    }
}

fn move_bodies(
    time: Res<Time>,
    mut query: Query<(&Distance, &OrbitAngularVelocity, &mut Transform), With<Mesh2d>>,
) {
    for (distance_to_origin, orbit_angular_velocity, mut transform) in query.iter_mut() {
        let theta = orbit_angular_velocity.0 * time.elapsed_secs();
        let x = distance_to_origin.0 * theta.cos();
        let y = distance_to_origin.0 * theta.sin();
        transform.translation = Vec3::new(x, y, 0.0);
    }
}

fn spawn_circle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    circle_info: CircleInfo,
) {
    let circle = meshes.add(Mesh::from(Circle::new(circle_info.radius)));
    let color = materials.add(circle_info.color);

    commands
        .spawn((
            circle_info.angular_velocity,
            circle_info.orbit_angular_velocity,
            Distance(circle_info.x), // Leave the distance signed can help rendering
            Mesh2d(circle),
            MeshMaterial2d(color),
            Transform::from_xyz(circle_info.x, 0.0, 0.0),
        ))
        .with_children(|parent| {
            let line = meshes.add(Mesh::from(Rectangle::new(circle_info.radius, 0.3)));
            parent.spawn((
                Mesh2d(line),
                MeshMaterial2d(circle_info.line_color),
                Transform::from_xyz(circle_info.radius * 0.5, 0.0, 0.0),
            ));
        });
}
