use bevy::prelude::*;

use crate::AudioEvent;

pub fn music_plugin(app: &mut App) {
    app.add_observer(play_music);
}

#[derive(Event)]
pub struct MusicEvent;

fn play_music(_: Trigger<MusicEvent>, mut commands: Commands, mut is_playing: Local<bool>) {
    if *is_playing {
        return;
    }

    *is_playing = true;
    commands.trigger(AudioEvent {
        sample: "aster.ogg",
        volume: 0.40,
        speed: 0.80,
        looping: true,
        ..Default::default()
    });
}
