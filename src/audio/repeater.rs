use bevy::prelude::*;
use std::time::Duration;

use crate::audio::AudioEvent;

pub fn repeater_plugin(app: &mut App) {
    app.add_systems(Update, handle_repeaters);
}

#[derive(Component)]
pub struct SoundRepeater {
    timer: Timer,
    next_sound: Box<dyn FnMut() -> AudioEvent + Send + Sync + 'static>,
    next_duration: Box<dyn FnMut() -> Duration + Send + Sync + 'static>,
}

impl SoundRepeater {
    pub fn new(
        sound: impl FnMut() -> AudioEvent + Send + Sync + 'static,
        mut duration: impl FnMut() -> Duration + Send + Sync + 'static,
    ) -> Self {
        Self {
            next_sound: Box::new(sound),
            timer: Timer::new(duration(), TimerMode::Repeating),
            next_duration: Box::new(duration),
        }
    }
}

fn handle_repeaters(mut q: Query<&mut SoundRepeater>, mut commands: Commands, time: Res<Time>) {
    let delta = time.delta();

    for mut repeater in &mut q {
        if repeater.timer.tick(delta).just_finished() {
            commands.trigger((repeater.next_sound)());
            let next_duration = (repeater.next_duration)();
            repeater.timer.set_duration(next_duration);
        }
    }
}
