use bevy::prelude::*;

#[derive(Component)]
struct TimerDisplay;

pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, insert_timer_display)
            .add_systems(Update, display_time);
    }
}

fn insert_timer_display(mut commands: Commands) {
    commands.spawn((
        Text::new("00:00:00"),
        TextColor(Color::srgb_u8(40, 0, 97)),
        TimerDisplay,
    ));
}

fn display_time(time: Res<Time>, mut query: Query<&mut Text, With<TimerDisplay>>) {
    for mut text in query.iter_mut() {
        let seconds = time.elapsed_secs();
        text.0 = format!("Time: {}", format_seconds(seconds));
    }
}

fn format_seconds(secs: f32) -> String {
    let total_seconds = secs as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = secs % 60.0;
    format!("{:02}:{:02}:{:02.3}", hours, minutes, seconds)
}
