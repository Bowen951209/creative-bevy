use bevy::{
    core_pipeline::Skybox,
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};

/// A plugin that reinterprets the cubemap resource image if needed and attaches it to all skybox entities.
pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, asset_loaded);
    }
}

#[derive(Resource)]
pub struct Cubemap {
    is_loaded: bool,
    image_handle: Handle<Image>,
}

impl Cubemap {
    pub fn new(image_handle: Handle<Image>) -> Self {
        Self {
            is_loaded: false,
            image_handle,
        }
    }
}

fn asset_loaded(
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut cubemap: ResMut<Cubemap>,
    mut skyboxes: Query<&mut Skybox>,
) {
    if !cubemap.is_loaded && asset_server.load_state(&cubemap.image_handle).is_loaded() {
        let image = images.get_mut(&cubemap.image_handle).unwrap();
        // NOTE: PNGs do not have any metadata that could indicate they contain a cubemap texture,
        // so they appear as one texture. The following code reconfigures the texture as necessary.
        if image.texture_descriptor.array_layer_count() == 1 {
            info!(
                "Cubemap image {} has array layer count of 1; reinterpreting as a cubemap texture.",
                cubemap.image_handle.id()
            );

            image.reinterpret_stacked_2d_as_array(6);
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
        }

        for mut skybox in &mut skyboxes {
            skybox.image = cubemap.image_handle.clone();
        }

        cubemap.is_loaded = true;
    }
}
