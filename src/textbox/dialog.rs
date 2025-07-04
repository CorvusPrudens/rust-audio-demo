use crate::footsteps::WalkEvent;

use super::sequence::{CharacterFragment, spawn_textbox_sequence};
use bevy::prelude::*;
use bevy_sequence::prelude::{FragmentExt, IntoFragment};

use super::sequence::AudioSequence;

pub fn dialog_plugin(app: &mut App) {
    app.add_systems(Startup, |mut commands: Commands| {
        spawn_textbox_sequence(meeting(), &mut commands);
    });
}

fn meeting() -> impl IntoFragment<AudioSequence> {
    (
        3.0,
        "It's a gentle night.".narrator(),
        "The moon peeks behind the clouds.",
        "The wind blows through the tall trees.",
        1.5,
        "You see someone walking towards you.".on_start(|mut commands: Commands| {
            commands.trigger(WalkEvent { volume: 0.5 });
        }),
        "Oh no<0.2>... [1]<1>he wants to `talk to you`[shaky 0.01]..."
            .on_start(|mut commands: Commands| {
                // Toggle off and back on again.
                commands.trigger(WalkEvent::default());
                commands.trigger(WalkEvent { volume: 0.75 });
            })
            .on_end(|mut commands: Commands| {
                commands.trigger(WalkEvent::default());
                commands.trigger(WalkEvent { volume: 1.0 });
            }),
        1.0.on_end(|mut commands: Commands| {
            commands.trigger(WalkEvent::default());
        }),
        2.0,
        "Hey there!".stranger(),
        "Lovely night, isn't it~",
        "Maybe not with people like this around...".narrator(),
        "Do you mind if I join you?".stranger(),
        1.0,
        "You can't think of an excuse, so you begrudgingly accept."
            .narrator()
            .on_end(|mut commands: Commands| {
                commands.trigger(WalkEvent { volume: 1.0 });
            }),
    )
}
