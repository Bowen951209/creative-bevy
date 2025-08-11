use bevy::{core_pipeline::Skybox, pbr::CascadeShadowConfigBuilder, prelude::*};
use bevy_flycam::{FlyCam, KeyBindings, prelude::NoCameraPlayerPlugin};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_rapier3d::prelude::*;
use creative_bevy::plugins::{
    esc_exit_plugin::EscExitPlugin,
    skybox_plugin::{Cubemap, SkyboxPlugin},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EscExitPlugin,
            SkyboxPlugin,
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
        .add_systems(Update, insert_physics)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let cubemap_image_handle = asset_server.load("textures/Ryfjallet_cubemap.png");

    let cubemap = Cubemap::new(cubemap_image_handle.clone());
    commands.insert_resource(cubemap);

    commands.spawn((
        Camera3d::default(),
        Skybox {
            image: cubemap_image_handle,
            brightness: 1000.0,
            ..Default::default()
        },
        FlyCam,
    ));

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
}

fn insert_physics(
    mut command: Commands,
    mut ran: Local<bool>,
    meshes: Res<Assets<Mesh>>,
    query: Query<(Entity, &Name, &Mesh3d)>,
) {
    if *ran {
        return;
    }

    let mut sum = 0;
    for (entity, _, mesh3d) in query
        .iter()
        .filter(|(_, name, _)| name.starts_with("collider_"))
    {
        let mesh = meshes.get(mesh3d.id()).unwrap();

        let collider = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::default()).unwrap();

        command
            .entity(entity)
            .insert((RigidBody::Dynamic, collider, Restitution::new(0.8)));
        sum += 1;
    }

    if sum != 0 {
        info!("Spawned {sum} colliders");
        *ran = true;
    }
}
