use bevy::{core_pipeline::Skybox, pbr::CascadeShadowConfigBuilder, prelude::*};
use bevy_flycam::{FlyCam, KeyBindings, prelude::NoCameraPlayerPlugin};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_rapier3d::prelude::*;
use creative_bevy::plugins::{
    esc_exit_plugin::EscExitPlugin,
    skybox_plugin::{Cubemap, SkyboxPlugin},
    third_person_camera_plugin::{ThirdPersonCamera, ThirdPersonCameraPlugin},
};

const THIRD_PERSON_CAMERA_SENSITIVITY: f32 = 0.000002;

#[derive(Component)]
struct Controller;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EscExitPlugin,
            SkyboxPlugin,
            ThirdPersonCameraPlugin,
            NoCameraPlayerPlugin,
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
        ))
        .insert_resource(KeyBindings {
            toggle_grab_cursor: KeyCode::F1,
            ..Default::default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                insert_physics,
                control_ball,
                activate_fly_camera,
                activate_third_person_camera,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // This is a relatively small scene, so use tighter shadow
        // cascade bounds than the default for better quality.
        // We also adjusted the shadow map to be larger since we're
        // only using a single cascade.
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 1.6,
            ..default()
        }
        .build(),
    ));

    let scene_handle = asset_server.load::<Scene>("models/suzanne_with_ring.gltf#Scene0");

    commands.spawn(SceneRoot(scene_handle));

    let ball_radius = 0.5;

    let ball = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(ball_radius))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb_u8(190, 246, 250),
                metallic: 0.0,
                perceptual_roughness: 1.0,
                ..default()
            })),
            Controller,
            Name::new("Ball"),
            RigidBody::Dynamic,
            Transform::from_xyz(0.0, 1.0, 0.0),
            Collider::ball(ball_radius),
        ))
        .id();

    let cubemap_image_handle = asset_server.load("textures/Ryfjallet_cubemap.png");
    let cubemap = Cubemap::new(cubemap_image_handle.clone());
    commands.insert_resource(cubemap);

    commands.spawn((
        ThirdPersonCamera {
            follow_entity: ball,
            distance: 4.0,
            sensitivity: THIRD_PERSON_CAMERA_SENSITIVITY,
        },
        Camera3d::default(),
        Skybox {
            image: cubemap_image_handle,
            brightness: 1000.0,
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(0.0, 2.0, 5.0)),
    ));
}

fn insert_physics(
    mut command: Commands,
    mut ran: Local<bool>,
    meshes: Res<Assets<Mesh>>,
    query: Query<(&ChildOf, &Name, &Mesh3d)>,
) {
    if *ran {
        return;
    }

    let mut sum = 0;
    for (child_of, _, mesh3d) in query
        .iter()
        .filter(|(_, name, _)| name.starts_with("collider_"))
    {
        let mesh = meshes.get(mesh3d.id()).unwrap();

        let collider = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::default()).unwrap();

        // Insert the physics components to the entity's parent, not the entity itself
        command
            .entity(child_of.0)
            .insert((RigidBody::Dynamic, collider, Restitution::new(0.8)));

        sum += 1;
    }

    if sum != 0 {
        info!("Spawned {sum} colliders");
        *ran = true;
    }
}

fn control_ball(
    mut command: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_transform_query: Query<&Transform, With<ThirdPersonCamera>>,
    ball_query: Query<Entity, (With<RigidBody>, With<Controller>)>,
) {
    let Ok(camera_transform) = camera_transform_query.single() else {
        return;
    };

    let force_scale = 2.0;

    for entity in ball_query.iter() {
        let direction_xyz = if keyboard_input.pressed(KeyCode::KeyW) {
            camera_transform.forward().as_vec3()
        } else if keyboard_input.pressed(KeyCode::KeyS) {
            camera_transform.back().as_vec3()
        } else if keyboard_input.pressed(KeyCode::KeyA) {
            camera_transform.left().as_vec3()
        } else if keyboard_input.pressed(KeyCode::KeyD) {
            camera_transform.right().as_vec3()
        } else {
            command.entity(entity).insert(ExternalForce {
                force: Vec3::ZERO,
                ..Default::default()
            });
            continue;
        };

        let direction_xz = vec3(direction_xyz.x, 0.0, direction_xyz.z).normalize() * force_scale;

        command.entity(entity).insert(ExternalForce {
            force: direction_xz,
            ..Default::default()
        });
    }
}

fn activate_third_person_camera(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Query<Entity, With<FlyCam>>,
    ball_query: Query<Entity, (With<RigidBody>, With<Controller>)>,
) {
    let ball = match ball_query.single() {
        Ok(ball) => ball,
        Err(_) => {
            warn!("Ball not found!");
            return;
        }
    };

    if keyboard_input.just_pressed(KeyCode::Digit1) {
        info!("Activating third-person camera");

        for entity in camera_query.iter() {
            commands
                .entity(entity)
                .remove::<FlyCam>()
                .insert(ThirdPersonCamera {
                    follow_entity: ball,
                    distance: 4.0,
                    sensitivity: THIRD_PERSON_CAMERA_SENSITIVITY,
                });
        }
    }
}

fn activate_fly_camera(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Query<Entity, With<ThirdPersonCamera>>,
) {
    if keyboard_input.just_pressed(KeyCode::Digit2) {
        info!("Activating fly camera");

        for entity in camera_query.iter() {
            commands
                .entity(entity)
                .remove::<ThirdPersonCamera>()
                .insert(FlyCam);
        }
    }
}
