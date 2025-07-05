use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

use crate::audio::{AudioEvent, VolumeFadeEvent, repeater::SoundRepeater};

mod sequences;

pub fn narrative_plugin(app: &mut App) {
    app.add_plugins(sequences::sequences_plugin)
        .add_systems(Startup, startup);
}

fn startup(mut commands: Commands) {
    let fade_in_time = 2.5;

    commands.trigger(AudioEvent {
        sample: "pine_trees.ogg",
        looping: true,
        name: Some("pine"),
        volume: 0.0,
        ..Default::default()
    });

    // We fade in the ambience for a nice startup vibe
    commands.trigger(VolumeFadeEvent {
        name: "pine",
        start: 0.0,
        end: 1.1,
        seconds: fade_in_time,
    });

    commands.trigger(AudioEvent {
        sample: "nightingale.ogg",
        looping: true,
        position: Some(Vec2::new(15.0, 10.0)),
        volume: 0.0,
        name: Some("nightingale"),
        ..Default::default()
    });

    commands.trigger(VolumeFadeEvent {
        name: "nightingale",
        start: 0.0,
        end: 0.9,
        seconds: fade_in_time,
    });

    commands.spawn(SoundRepeater::new(
        || AudioEvent {
            sample: "caw.ogg",
            position: Some(Vec2::new(-15.0, 15.0)),
            ..Default::default()
        },
        || {
            let mut rng = rand::thread_rng();
            let duration = rng.gen_range(10.0..25.0);

            Duration::from_secs_f32(duration)
        },
    ));
}
