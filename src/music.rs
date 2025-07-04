use bevy::prelude::*;

use crate::audio_events::AudioEvent;

pub fn music_plugin(app: &mut App) {
    app.add_observer(play_music);
}

#[derive(Event, Clone)]
pub struct MusicEvent;

fn play_music(_: Trigger<MusicEvent>, mut commands: Commands, mut is_playing: Local<bool>) {
    if *is_playing {
        return;
    }

    *is_playing = true;
    commands.trigger(AudioEvent {
        sample: "aster.ogg",
        volume: 0.52,
        speed: 0.80,
        looping: true,
        name: Some("music"),
        ..Default::default()
    });
}
