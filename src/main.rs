use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};

#[derive(Component)]
struct Body;

#[derive(Component)]
struct AngularVelocity(f32);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((DefaultPlugins, PanCamPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (exit_on_esc, rotate_bodies))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d, PanCam::default()));

    let radii = [50.0];
    let colors = [(1.0, 0.0, 0.0)];

    let line_color = materials.add(Color::WHITE);

    for (radius, color) in radii.into_iter().zip(colors) {
        let circle = meshes.add(Mesh::from(Circle::new(radius)));
        let color = materials.add(Color::srgb(color.0, color.1, color.2));

        commands
            .spawn((
                Body,
                AngularVelocity(0.8),
                Mesh2d(circle),
                MeshMaterial2d(color),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ))
            .with_children(|parent| {
                let line = meshes.add(Mesh::from(Rectangle::new(radius, 1.0)));
                parent.spawn((
                    Mesh2d(line),
                    MeshMaterial2d(line_color.clone()),
                    Transform::from_xyz(radius * 0.5, 0.0, 0.0),
                ));
            });
    }
}

fn rotate_bodies(
    time: Res<Time>,
    mut query: Query<(&AngularVelocity, &mut Transform), With<Body>>,
) {
    for (angular_velocity, mut transform) in query.iter_mut() {
        transform.rotate(Quat::from_rotation_z(
            angular_velocity.0 * time.delta_secs(),
        ));
    }
}

fn exit_on_esc(keyboard_input: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        info!("Exiting application on Escape key press.");
        exit.write(AppExit::Success);
    }
}
