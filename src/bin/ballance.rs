use bevy::{core_pipeline::Skybox, prelude::*};
use bevy_flycam::{FlyCam, KeyBindings, prelude::NoCameraPlayerPlugin};
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
        ))
        .insert_resource(KeyBindings {
            toggle_grab_cursor: KeyCode::F1,
            ..Default::default()
        })
        .add_systems(Startup, setup)
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
}
