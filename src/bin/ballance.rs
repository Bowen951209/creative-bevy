use std::f32::consts::PI;

use bevy::{audio::Volume, core_pipeline::Skybox, pbr::CascadeShadowConfigBuilder, prelude::*};
use bevy_flycam::{FlyCam, KeyBindings, prelude::NoCameraPlayerPlugin};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_rapier3d::prelude::*;
use bevy_scene_hot_reloading::SceneHotReloadingPlugin;
use creative_bevy::plugins::{
    esc_exit_plugin::EscExitPlugin,
    skybox_plugin::{Cubemap, SkyboxPlugin},
    third_person_camera_plugin::{ThirdPersonCamera, ThirdPersonCameraPlugin},
};

const THIRD_PERSON_CAMERA_SENSITIVITY: f32 = 0.000002;

#[derive(Component)]
struct Controller;

#[derive(Component)]
struct Ball;

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
            // SceneHotReloadingPlugin is a temporary fix for a scene hot reloading bug in Bevy.
            // This issue is fixed in the main branch. When we upgrade to Bevy 0.17,
            // we can remove this plugin. See: https://github.com/bevyengine/bevy/pull/18358
            SceneHotReloadingPlugin,
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
                ball_sound,
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
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
    ));

    let scene_handle = asset_server.load::<Scene>("levels/level0/level0.gltf#Scene0");

    commands.spawn(SceneRoot(scene_handle));

    let ball_radius = 0.5;
    let ball = commands
        .spawn((
            Ball,
            Mesh3d(
                meshes.add(
                    Mesh::from(Sphere::new(ball_radius))
                        .with_generated_tangents() // for normal map & depth map
                        .unwrap(),
                ),
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(
                    asset_server.load("textures/broken_brick_wall/broken_brick_wall_diff_4k.png"),
                ),
                depth_map: Some(
                    asset_server.load("textures/broken_brick_wall/broken_brick_wall_disp_4k.png"),
                ),
                parallax_depth_scale: 0.01,
                normal_map_texture: Some(
                    asset_server.load("textures/broken_brick_wall/broken_brick_wall_nor_gl_4k.png"),
                ),
                diffuse_transmission_texture: Some(
                    asset_server.load("textures/broken_brick_wall/broken_brick_wall_diff_4k.png"),
                ),
                occlusion_texture: Some(
                    asset_server.load("textures/broken_brick_wall/broken_brick_wall_ao_4k.png"),
                ),
                metallic_roughness_texture: Some(
                    asset_server.load("textures/broken_brick_wall/broken_brick_wall_rough_4k.png"),
                ),
                ..default()
            })),
            Controller,
            RigidBody::Dynamic,
            Transform::from_xyz(0.0, 1.0, 0.0),
            Collider::ball(ball_radius),
            ExternalForce::default(),
            Damping {
                linear_damping: 0.1,
                angular_damping: 1.0,
            },
            Velocity::default(),
            ActiveEvents::COLLISION_EVENTS,
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

/// This system adds physics components to the parents of meshes imported from glTF whose names start with "collider_".
/// It runs only once, one frame after the scene is loaded.
/// Note: We intentionally delay execution by one frame after loading because [`ChildOf`] components are not yet available immediately after the scene loads.
fn insert_physics(
    mut commands: Commands,
    mut scene_events: EventReader<AssetEvent<Scene>>,
    meshes: Res<Assets<Mesh>>,
    mesh_query: Query<(&ChildOf, &Name, &Mesh3d)>,
    mut should_run: Local<bool>,
) {
    for event in scene_events.read() {
        let AssetEvent::LoadedWithDependencies { id: _ } = event else {
            *should_run = true;
            return;
        };
    }

    if !*should_run {
        return;
    }

    let mut sum = 0;
    for (child_of, _, mesh3d) in mesh_query
        .iter()
        .filter(|(_, name, _)| name.starts_with("collider_"))
    {
        let mesh = meshes.get(mesh3d.id()).unwrap();
        let collider = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::default()).unwrap();

        // Insert the physics components to the entity's parent, not the entity itself
        commands.entity(child_of.parent()).insert((
            RigidBody::Fixed,
            collider,
            Restitution::new(0.8),
        ));

        sum += 1;
    }

    info!("Inserted {sum} colliders");
    *should_run = false;
}

fn control_ball(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_transform_query: Query<&Transform, With<ThirdPersonCamera>>,
    mut query: Query<&mut ExternalForce, With<Ball>>,
) {
    let Ok(camera_transform) = camera_transform_query.single() else {
        return;
    };

    let force_scale = 1.0;

    for mut external_force in query.iter_mut() {
        let mut direction = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::KeyW) {
            // direction += xz_normalize(camera_transform.forward().as_vec3());
            direction += camera_transform.left().as_vec3();
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction += camera_transform.right().as_vec3();
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction += camera_transform.back().as_vec3();
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction += camera_transform.forward().as_vec3();
        }

        external_force.torque = direction * force_scale;
    }
}

/// Sets the ball's sound volume according to its velocity.
/// The sound is muted when the ball is not in contact with anything.
/// This system will insert audio components for you; do not insert them manually when creating the ball.
/// Otherwise, a short period of sound may play even if the ball is not in contact with anything.
fn ball_sound(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut collision_events: EventReader<CollisionEvent>,
    mut query: Query<(&Velocity, &mut AudioSink), With<Ball>>,
) {
    // If the ball is not in contact with anything, mute the sound; otherwise, unmute it.
    // We listen to collision events to determine this.
    // We also insert an `AudioPlayer` component if it doesn't exist.
    for event in collision_events.read() {
        let (entity, is_started) = match event {
            CollisionEvent::Started(entity, _, _) => (entity, true),
            CollisionEvent::Stopped(entity, _, _) => (entity, false),
        };

        match query.get_mut(*entity) {
            Ok((_, mut sink)) => {
                if is_started {
                    sink.unmute();
                } else {
                    sink.mute();
                }
            }
            Err(_) => {
                // Audio components don't exist, insert them
                commands.entity(*entity).insert((
                    AudioPlayer::new(asset_server.load("sounds/stones-falling-6375.ogg")),
                    PlaybackSettings::LOOP,
                ));
            }
        }
    }

    // Set the volume based on the ball's velocity. If the ball is muted, don't process.
    for (velocity, mut sink) in query.iter_mut().filter(|q| !q.1.is_muted()) {
        sink.set_volume(Volume::Linear(velocity.linvel.length() * 0.4));
    }
}

fn activate_third_person_camera(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Query<Entity, With<FlyCam>>,
    ball_query: Query<Entity, With<Ball>>,
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
