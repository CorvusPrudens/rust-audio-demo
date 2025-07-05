//! All the demo's scripted sequences.
//!
//! The sequences are written with a behavior-tree-like framework I'm working on.
//! The details of how it works aren't important, but it makes it easy to
//! tightly integrated arbitrary events like audio queues with sequences
//! of dialog (or anything else, really).
//!
//! Nothing about the sequences changes between engines, so any differences
//! are down to how the engines are integrated and their implementation details.

use bevy::{color::palettes, prelude::*, time::Stopwatch};
use bevy_pretty_text::style::StyleAppExt;
use bevy_sequence::prelude::{FragmentExt, IntoFragment, spawn_root};

use crate::{
    audio::{
        AudioEvent, VolumeFadeEvent,
        chimes::{ChimesEnable, ChimesTimer},
        footsteps::WalkEvent,
    },
    textbox::sequence::{AudioSequence, CharacterFragment, despawn_textbox, dynamic},
};

pub fn sequences_plugin(app: &mut App) {
    app.add_systems(Startup, |mut commands: Commands| {
        spawn_root(demo().always().once(), &mut commands);
    })
    .add_systems(Update, tick_watch)
    .register_pretty_style("yellow", |_| Color::from(palettes::basic::YELLOW));
}

fn demo() -> impl IntoFragment<AudioSequence> {
    (
        intro().on_end(despawn_textbox),
        creek().on_end(despawn_textbox),
        end().on_end(despawn_textbox),
    )
}

fn intro() -> impl IntoFragment<AudioSequence> {
    (
        3.0,
        "It's a gentle night.".narrator(),
        "The moon peeks behind the clouds.",
        "The wind blows through the tall trees.",
        1.5,
        "You see someone walking towards you.".on_start(trigger(WalkEvent::Start(0.5))),
        "Oh no<0.2>... [1]<1>he wants to <0.5>`talk to you`[shake 0.01]..."
            .on_start(trigger(WalkEvent::Start(0.75)))
            .on_end(trigger(WalkEvent::Start(1.0))),
        1.5.on_end(trigger(WalkEvent::Stop)),
        1.5,
        "Hey there!".stranger().on_end(|mut commands: Commands| {
            commands.spawn(AsterWatch {
                timer: Stopwatch::new(),
            });
        }),
        "Lovely night, isn't it~",
        "<0.2>...[1]<1> he seems too happy...".narrator(),
        "Mind if I join you?".stranger(),
        1.0,
        "You can't think of an excuse, [0.5]so unfortunately you have to accept."
            .narrator()
            .on_end(trigger(WalkEvent::Start(1.0))),
        3.0,
    )
}

fn creek() -> impl IntoFragment<AudioSequence> {
    (
        "My name's `Aster|yellow`.[1] Pleased to meet you!"
            .stranger()
            // queue the music!
            .on_end(trigger(AudioEvent {
                sample: "aster.ogg",
                volume: 0.52,
                speed: 0.80,
                looping: true,
                name: Some("music"),
                ..Default::default()
            })),
        2.5,
        "Aster runs his hand absent-mindedly though some chimes."
            .narrator()
            .on_start(|mut commands: Commands| {
                commands.spawn(ChimesTimer::new(0.65, Vec2::new(4.0, 3.0)));
            }),
        "(Who put chimes out here?)".on_start(trigger(VolumeFadeEvent {
            name: "pine",
            start: 1.1,
            end: 1.3,
            seconds: 5.0,
        })),
        3.0,
        "Don't you love the sound of pine trees in the wind?".aster(),
        "It almost sounds like<0.2>...[0.5]<1> I don't know,[0.5] a big[0.33] river or something.",
        4.0.on_start(|mut commands: Commands| {
            let name = "creek";

            commands.trigger(AudioEvent {
                sample: "creek.ogg",
                volume: 0.0,
                looping: true,
                name: Some(name),
                ..Default::default()
            });

            commands.trigger(VolumeFadeEvent {
                name,
                start: 0.0,
                end: 0.4,
                seconds: 5.0,
            });

            commands.trigger(VolumeFadeEvent {
                name: "pine",
                start: 1.3,
                end: 1.1,
                seconds: 5.0,
            });
        })
        .on_end(trigger(WalkEvent::Stop)),
        "Oh look![1] A `little`[wave] river!".aster(),
        "Aster deftly crosses the stream,[0.5] prancing between the little rocks.".narrator(),
        1.0,
        "Now it's your turn.[1]<0.5> `Oh man...`[shake]",
        1.5.on_end(trigger(AudioEvent {
            sample: "splash.ogg",
            volume: 1.0,
            ..Default::default()
        })),
        0.7,
        "Oh no!".aster(),
        "Naturally, you slipped on the last rock.[0.5] Aster helps pull you out."
            .narrator()
            .on_end(trigger(VolumeFadeEvent {
                name: "creek",
                start: 0.4,
                end: 0.30,
                seconds: 4.0,
            })),
        1.0,
    )
}

fn end() -> impl IntoFragment<AudioSequence> {
    (
        "Here,[0.5] I always bring this just in case."
            .aster()
            .on_end(trigger(AudioEvent {
                sample: "zipper.ogg",
                volume: 0.7,
                ..Default::default()
            })),
        1.5,
        "He fishes around in his bag for a moment,[0.5] and hands you a towel.".narrator(),
        "You dry yourself off, wondering what kind of contingencies Aster's planning for."
            .on_start(trigger(AudioEvent {
                sample: "towel.ogg",
                ..Default::default()
            })),
        2.0,
        "You go to hand the towel back,[0.5] except<0.2>...[1] <1>you don't [0.5]see him anywhere."
            .on_start(trigger(VolumeFadeEvent {
                name: "music",
                start: 0.52,
                end: 0.0,
                seconds: 6.0,
            })),
        2.0,
        "Huh...",
        2.0,
        "Maybe he[0.5] went home.",
        dynamic(|watch: Query<&AsterWatch>| {
            let time = watch
                .single()
                .map(|w| w.timer.elapsed().as_secs_f32() as i32)
                .unwrap_or(30);

            format!(
                "You feel a little sad that he's gone, even though you only knew him for like [0.7]{time} seconds..."
            )
        }),
        3.0,
        "Well,[1] you had best head home too.",
        "(That's all! You can also mess with the chimes by pressing C.)".on_start(
            |mut commands: Commands| {
                commands.spawn(ChimesEnable);
            },
        ),
    )
}

/// A convenience HOF for triggering events.
fn trigger<E: Event + Clone>(event: E) -> impl Fn(Commands) + Send + Sync + 'static {
    move |mut commands: Commands| {
        commands.trigger(event.clone());
    }
}

/// I thought this would be a cute gag.
#[derive(Component)]
struct AsterWatch {
    timer: Stopwatch,
}

fn tick_watch(mut watches: Query<&mut AsterWatch>, time: Res<Time>) {
    for mut watch in &mut watches {
        watch.timer.tick(time.delta());
    }
}
