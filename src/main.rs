use std::time::Duration;

use bevy::{ecs::schedule::ExecutorKind, prelude::*};
use clap::{Parser, ValueEnum};
use rand::Rng;

mod audio_events;
mod chimes;
mod engine;
mod footsteps;
mod music;
mod repeater;
mod textbox;

/// Evaluate the Firewheel and `rodio` audio engines
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Select the engine to evaluate
    engine: Engine,
}

#[derive(ValueEnum, Clone, Debug)]
enum Engine {
    Firewheel,
    Rodio,
}

fn main() {
    let args = Args::parse();
    bevy::ecs::error::GLOBAL_ERROR_HANDLER
        .set(bevy::ecs::error::warn)
        .unwrap();

    let mut app = App::new();

    app.insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)))
        .add_plugins((
            DefaultPlugins
                .set(TaskPoolPlugin {
                    task_pool_options: TaskPoolOptions {
                        max_total_threads: 1,
                        ..Default::default()
                    },
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "night in a pine forest".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(ImagePlugin::default_linear()),
            chimes::chimes_plugin,
            repeater::repeater_plugin,
            footsteps::footsteps_plugin,
            music::music_plugin,
            textbox::textbox_plugin,
            audio_events::audio_events_plugin,
        ));

    match args.engine {
        Engine::Firewheel => {
            app.add_plugins(engine::firewheel_engine::FirewheelPlugin);
        }
        Engine::Rodio => {
            app.add_plugins(engine::rodio_engine::RodioPlugin);
        }
    }

    app.add_systems(Startup, |mut commands: Commands| {
        commands.trigger(audio_events::AudioEvent {
            sample: "pine_trees.ogg",
            looping: true,
            name: Some("pine"),
            volume: 0.0,
            ..Default::default()
        });

        commands.trigger(audio_events::VolumeFadeEvent {
            name: "pine",
            start: 0.0,
            end: 1.1,
            seconds: 3.0,
        });

        commands.trigger(audio_events::AudioEvent {
            sample: "nightingale.ogg",
            looping: true,
            position: Some(Vec2::new(15.0, 10.0)),
            ..Default::default()
        });

        commands.spawn(repeater::SoundRepeater::new(
            || audio_events::AudioEvent {
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

        commands.spawn(Camera2d);
    })
    // Just to simplify things a bit, we'll do single-threaded execution.
    .edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    })
    .edit_schedule(Update, |schedule| {
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    })
    .edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    })
    .run();
}
