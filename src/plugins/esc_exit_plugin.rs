use bevy::prelude::*;

pub struct EscExitPlugin;

impl Plugin for EscExitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, exit_on_esc);
    }
}

fn exit_on_esc(keyboard_input: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        info!("Exiting application on Escape key press.");
        exit.write(AppExit::Success);
    }
}
