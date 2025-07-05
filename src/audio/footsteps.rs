use bevy::prelude::*;
use rand::{Rng, seq::SliceRandom};
use std::time::Duration;

use crate::audio::{AudioEvent, repeater::SoundRepeater};

pub fn footsteps_plugin(app: &mut App) {
    app.add_observer(toggle_walking);
}

/// The walk event allows us to control footsteps, which are really
/// just a [`SoundRepeater`].
#[derive(Event, Clone)]
pub enum WalkEvent {
    Start(f32),
    Stop,
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
    if let Ok(walking) = walking.single() {
        commands.entity(walking).despawn();
    }

    match *trigger {
        WalkEvent::Start(volume) => {
            let mut rng = rand::thread_rng();
            let mut last_sound = *FOOTSTEPS.choose(&mut rng).unwrap();

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

            commands.spawn((
                Footsteps,
                SoundRepeater::new(next_sound, || {
                    let mut rng = rand::thread_rng();
                    let delay = rng.gen_range(0.9..1.1);

                    Duration::from_secs_f32(delay)
                }),
            ));
        }
        WalkEvent::Stop => {}
    }
}
