use bevy::prelude::*;
use clap::{Parser, ValueEnum};

mod audio;
mod engine;
mod narrative;
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
            // This helps minimize the potential for input latency
            bevy_framepace::FramepacePlugin,
            audio::audio_plugin,
            textbox::textbox_plugin,
            narrative::narrative_plugin,
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
        commands.spawn(Camera2d);
    })
    .run();
}
