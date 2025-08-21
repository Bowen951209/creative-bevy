use std::f32::consts::PI;

use bevy::{
    audio::Volume, core_pipeline::Skybox, input::common_conditions::input_toggle_active,
    pbr::CascadeShadowConfigBuilder, prelude::*,
};
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
struct Ball {
    radius: f32,
    is_in_bounds: bool,
}

impl Ball {
    fn new(radius: f32) -> Self {
        Self {
            radius,
            is_in_bounds: true,
        }
    }
}

#[derive(Component)]
struct Goal;

#[derive(Component)]
struct RestartPosition(Vec3);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EscExitPlugin,
            SkyboxPlugin,
            ThirdPersonCameraPlugin,
            NoCameraPlayerPlugin,
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F2)),
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
                insert_goal,
                detect_goal,
                rotate_goal,
                control_ball,
                ball_sound,
                detect_out_of_bounds,
                activate_fly_camera,
                activate_third_person_camera,
                restart,
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
    let ball_position = vec3(0.0, 1.0, 0.0);
    let ball = commands
        .spawn((
            Ball::new(ball_radius),
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
            Transform::from_translation(ball_position),
            RestartPosition(ball_position),
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
    ball_query: Query<(Entity, &Ball), (With<Ball>, Without<Collider>)>,
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
    // Insert physics to parents
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
    info!("Inserted {sum} physics from scene");

    let mut sum = 0;
    for (entity, ball) in ball_query.iter() {
        commands.entity(entity).insert((
            RigidBody::Dynamic,
            Collider::ball(ball.radius),
            ExternalForce::default(),
            Damping {
                linear_damping: 0.1,
                angular_damping: 1.0,
            },
            Velocity::default(),
            ActiveEvents::COLLISION_EVENTS,
        ));
        sum += 1;
    }
    info!("Inserted physics for {sum} balls");

    *should_run = false;
}

fn insert_goal(
    mut commands: Commands,
    mut scene_events: EventReader<AssetEvent<Scene>>,
    meshes: Res<Assets<Mesh>>,
    query: Query<(&ChildOf, &Name, &Mesh3d)>,
) {
    for event in scene_events.read() {
        let AssetEvent::LoadedWithDependencies { id: _ } = event else {
            return;
        };
    }

    for (child_of, _, mesh3d) in query
        .iter()
        .filter(|(_, name, _)| name.starts_with("goal_"))
    {
        let mesh = meshes.get(mesh3d.id()).unwrap();
        let collider = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::default()).unwrap();

        commands
            .entity(child_of.parent())
            .insert((Goal, collider, Sensor));
    }
}

fn detect_goal(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    asset_server: Res<AssetServer>,
    query: Query<(), With<Goal>>,
) {
    for event in collision_events.read() {
        let CollisionEvent::Started(entity, _, _) = event else {
            continue;
        };

        if !query.contains(*entity) {
            continue; // Not a goal, skip
        }

        info!("Goal reached by entity: {:?}", entity);
        commands.spawn((
            AudioPlayer::new(asset_server.load("sounds/mixkit-guitar-stroke-down-slow-2339.ogg")),
            PlaybackSettings::DESPAWN,
        ));

        commands.spawn((
            Text::new("You Win!"),
            TextFont::from_font_size(30.0),
            TextShadow::default(),
            TextLayout::new_with_justify(JustifyText::Center),
            Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                ..default()
            },
        ));
    }
}

/// Rotate the goal around its Y-axis
fn rotate_goal(mut query: Query<&mut Transform, With<Goal>>) {
    for mut transform in query.iter_mut() {
        transform.rotate(Quat::from_rotation_y(0.1));
    }
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
    ball_query: Query<(), With<Ball>>,
) {
    // If the ball is not in contact with anything, mute the sound; otherwise, unmute it.
    // We listen to collision events to determine this.
    // We also insert an `AudioPlayer` component if it doesn't exist.
    for event in collision_events.read() {
        let (entity, is_started) = match event {
            CollisionEvent::Started(_, entity, _) => (entity, true),
            CollisionEvent::Stopped(_, entity, _) => (entity, false),
        };

        if !ball_query.contains(*entity) {
            continue; // Not a ball, skip
        }

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

/// Detect when the ball's y position drops below the "bottom" boundary entity.
/// Level designers can add an empty object named "bottom" in Blender to define the out-of-bounds threshold.
/// When the ball falls below this threshold, logs a message and displays "You Fall!" text, and play a trumpet sound.
/// If the "bottom" entity is missing, logs an error. These checks only run once per scene load to avoid repeated messages.
fn detect_out_of_bounds(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scene_events: EventReader<AssetEvent<Scene>>,
    bottom_query: Query<(&Transform, &Name)>,
    mut ball_query: Query<(&Transform, &mut Ball)>,
    mut should_run: Local<bool>,
) {
    for event in scene_events.read() {
        match event {
            AssetEvent::Modified { id: _ } => {
                // Scene is not loaded, shouldn't run
                *should_run = false;
            }
            AssetEvent::LoadedWithDependencies { id: _ } => {
                // Scene is loaded, should run
                *should_run = true;
            }
            _ => continue,
        }
    }

    if !*should_run {
        return;
    }

    let Some(bottom) = bottom_query
        .iter()
        .find(|(_, name)| name.as_str() == "bottom")
    else {
        error!("Bottom entity not found!");
        *should_run = false; // We only want to log this once
        return;
    };

    for (_, mut ball) in ball_query.iter_mut().filter(|(transform, ball)| {
        (transform.translation.y < bottom.0.translation.y) && ball.is_in_bounds
    }) {
        info!("A ball is out of bounds!");
        ball.is_in_bounds = false;

        commands
            .spawn((
                Text::new("You Fall!\n"),
                TextFont::from_font_size(30.0),
                TextShadow::default(),
                TextLayout::new_with_justify(JustifyText::Center),
                TextColor(Color::srgb_u8(168, 50, 98)),
                Node {
                    align_self: AlignSelf::Center,
                    justify_self: JustifySelf::Center,
                    ..default()
                },
            ))
            .with_child((
                TextSpan::new("Press R to restart"),
                TextColor(Color::srgb_u8(0, 130, 119)),
            ));

        commands.spawn((
            AudioPlayer::new(asset_server.load("sounds/cartoon-fail-trumpet-278822.ogg")),
            PlaybackSettings::DESPAWN,
        ));
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

/// Restarts the game when the player presses the R key.
///  - Teleports the ball back to its restart position (specified by the [`RestartPosition`] component) and resets its velocity.
///  - Plays a sound effect.
///  - If any fail text is on the screen, it will be despawned. This is necessary when restarting after a fall.
fn restart(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut ball_query: Query<(&mut Ball, &mut Transform, &mut Velocity, &RestartPosition)>,
    text_query: Query<(Entity, &Text)>,
) {
    if !keyboard_input.just_pressed(KeyCode::KeyR) {
        return;
    }

    info!("Teleporting the ball back to restart position");
    for (mut ball, mut transform, mut velocity, restart_position) in ball_query.iter_mut() {
        ball.is_in_bounds = true;
        transform.translation = restart_position.0;
        velocity.linvel = Vec3::ZERO;
        velocity.angvel = Vec3::ZERO;
    }

    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/owned-112942.ogg")),
        PlaybackSettings::DESPAWN,
    ));

    // Despawn the fail text
    if let Some(fail_text) = text_query
        .iter()
        .find(|(_, text)| text.as_str() == "You Fall!\n")
    {
        commands.entity(fail_text.0).despawn();
        info!("Fall text despawned");
    }
}
