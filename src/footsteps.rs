use std::time::Duration;

use bevy::prelude::*;
use rand::{Rng, seq::SliceRandom};

use crate::{audio_events::AudioEvent, repeater::SoundRepeater};

pub fn footsteps_plugin(app: &mut App) {
    app.add_observer(toggle_walking);
}

#[derive(Event, Default, Clone)]
pub struct WalkEvent {
    pub volume: f32,
}

#[derive(Component)]
struct Footsteps;

const FOOTSTEPS: &[&str] = &[
    "footsteps/step1.ogg",
    "footsteps/step2.ogg",
    "footsteps/step3.ogg",
    "footsteps/step4.ogg",
    "footsteps/step5.ogg",
    "footsteps/step6.ogg",
    "footsteps/step7.ogg",
    "footsteps/step8.ogg",
];

fn toggle_walking(
    trigger: Trigger<WalkEvent>,
    walking: Query<Entity, With<Footsteps>>,
    mut commands: Commands,
) {
    match walking.single() {
        Ok(walking) => {
            commands.entity(walking).despawn();
        }
        Err(_) => {
            let mut last_sound = FOOTSTEPS[0];
            let volume = trigger.volume;
            let mut next_sound = move || {
                let mut rng = rand::thread_rng();

                let speed = rng.gen_range(0.95..1.05);

                let options = FOOTSTEPS
                    .iter()
                    .filter(|s| **s != last_sound)
                    .collect::<Vec<_>>();

                let sample = **options.choose(&mut rng).unwrap();
                last_sound = sample;

                AudioEvent {
                    sample,
                    speed,
                    volume,
                    ..Default::default()
                }
            };

            commands.trigger(next_sound());
            // commands.trigger(MusicEvent);

            commands.spawn((
                Footsteps,
                SoundRepeater::new(next_sound, || {
                    let mut rng = rand::thread_rng();
                    let delay = rng.gen_range(0.9..1.1);

                    Duration::from_secs_f32(delay)
                }),
            ));
        }
    }
}
