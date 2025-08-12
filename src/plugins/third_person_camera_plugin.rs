use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

pub struct ThirdPersonCameraPlugin;

impl Plugin for ThirdPersonCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_camera);
    }
}

#[derive(Component)]
pub struct ThirdPersonCamera {
    pub follow_entity: Entity,
    pub distance: f32,
    pub sensitivity: f32,
}

fn update_camera(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut state: EventReader<MouseMotion>,
    mut cam_query: Query<(&ThirdPersonCamera, &mut Transform)>,
    trans_query: Query<&Transform, Without<ThirdPersonCamera>>,
) {
    let window = match primary_window.single() {
        Ok(w) => w,
        Err(_) => {
            warn!("Primary window not found!");
            return;
        }
    };

    for (camera, mut transform) in cam_query.iter_mut() {
        // The Euler conversion ensures the correct rotation behavior
        let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

        if window.cursor_options.grab_mode != CursorGrabMode::None {
            // Use smallest of height or width for consistent sensitivity
            let window_scale = window.height().min(window.width());
            let scale = camera.sensitivity * window_scale;

            for mouse_motion in state.read() {
                yaw -= scale * mouse_motion.delta.x;
                pitch -= scale * mouse_motion.delta.y;
            }
        }

        pitch = pitch.clamp(-1.54, 1.54);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

        if let Ok(target_transform) = trans_query.get(camera.follow_entity) {
            transform.translation =
                target_transform.translation + transform.back() * camera.distance;
        } else {
            error!("Camera following an entity that doesn't have a Transform component");
        }
    }
}
