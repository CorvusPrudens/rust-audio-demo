use bevy::{platform::collections::HashSet, prelude::*};
use rand::{Rng, seq::SliceRandom, thread_rng};
use std::time::Duration;

use crate::audio::AudioEvent;

pub fn chimes_plugin(app: &mut App) {
    app.add_systems(Update, (trigger_chimes, hit_chimes).chain());
}

#[derive(Component)]
pub struct ChimesEnable;

#[derive(Component)]
pub struct ChimesTimer {
    initial: bool,
    timer: Timer,
    amplitude: f32,
    position: Vec2,
    played_samples: HashSet<usize>,
}

const CHIMES: &[&str] = &[
    "chimes/chime-d1.ogg",
    "chimes/chime-d2.ogg",
    "chimes/chime-e1.ogg",
    "chimes/chime-e2.ogg",
    "chimes/chime-f1.ogg",
    "chimes/chime-f2.ogg",
    "chimes/chime-a1.ogg",
    "chimes/chime-a2.ogg",
    "chimes/chime-b1.ogg",
    "chimes/chime-b2.ogg",
    "chimes/chime-d4.ogg",
    "chimes/chime-d5.ogg",
];

impl ChimesTimer {
    pub fn new(initial_amplitude: f32, position: Vec2) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            initial: true,
            timer: Timer::new(Duration::from_secs_f32(rng.r#gen()), TimerMode::Repeating),
            amplitude: initial_amplitude,
            position,
            played_samples: HashSet::default(),
        }
    }
}

fn trigger_chimes(
    _: Single<(), With<ChimesEnable>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        commands.spawn(ChimesTimer::new(0.6, Vec2::new(10.0, 10.0)));
    }
}

fn hit_chimes(
    mut chimes: Query<(Entity, &mut ChimesTimer)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let delta = time.delta();
    for (entity, mut timer) in &mut chimes {
        if timer.initial || timer.timer.tick(delta).just_finished() {
            timer.initial = false;
            let mut rng = thread_rng();

            timer.amplitude -= 0.03;
            let new_duration = rng.gen_range(0.1..0.3);
            timer
                .timer
                .set_duration(Duration::from_secs_f32(new_duration));

            if timer.amplitude <= 0.15 {
                commands.entity(entity).despawn();
                continue;
            }

            if timer.played_samples.len() == CHIMES.len() {
                timer.played_samples.clear();
            }

            // get an unplayed sample
            let next_sample = (0..CHIMES.len())
                .filter(|i| !timer.played_samples.contains(i))
                .collect::<Vec<_>>()
                .choose(&mut rng)
                .copied()
                .unwrap();

            commands.trigger(AudioEvent {
                sample: CHIMES[next_sample],
                position: Some(timer.position),
                volume: timer.amplitude * 2.0,
                speed: 0.9,
                ..Default::default()
            });
        }
    }
}
