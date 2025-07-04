use crate::audio_events::{AudioEvent, VolumeFade};
use bevy::{platform::collections::HashMap, prelude::*};
use rodio::{
    DeviceTrait, Sink, Source, SpatialSink, buffer::SamplesBuffer, cpal::traits::HostTrait,
};
use walkdir::WalkDir;

pub struct RodioPlugin;

impl Plugin for RodioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(initialize_rodio)
            .add_systems(PreStartup, load_samples)
            .add_systems(Update, (apply_fades, monitor_sinks).chain())
            .add_observer(handle_sample_event);
    }
}

#[derive(Resource)]
pub struct RodioStreamHandle(rodio::OutputStreamHandle);

/// Here we initialize the rodio audio engine.
fn initialize_rodio(app: &mut App) {
    let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();

    app.insert_non_send_resource(stream)
        .insert_resource(RodioStreamHandle(stream_handle));
}

fn load_samples(mut commands: Commands) -> Result {
    let sample_rate = rodio::cpal::default_host()
        .default_output_device()
        .ok_or("unable to find default output device")?
        .default_output_config()?
        .sample_rate();
    let assets_path = std::path::Path::new("assets");

    let mut assets = HashMap::default();

    for asset_entry in WalkDir::new(assets_path).into_iter().filter_map(|e| e.ok()) {
        let string_name: String = asset_entry
            .path()
            .strip_prefix(assets_path)
            .unwrap()
            .to_string_lossy()
            .into();

        // We'll eagerly decode and resample to match Firewheel.
        let Ok(data) = symphonium::SymphoniumLoader::new().load_f32(
            asset_entry.path(),
            Some(sample_rate.0),
            Default::default(),
            None,
        ) else {
            continue;
        };

        let mut buffer = vec![0.0; data.frames() * data.channels()];

        // interleave the buffer
        for frame in 0..data.frames() {
            for channel in 0..data.channels() {
                let buffer_index = frame * data.channels() + channel;

                buffer[buffer_index] = data.data[channel][frame];
            }
        }

        assets.insert(
            string_name,
            SamplesBuffer::new(data.channels() as u16, data.sample_rate, buffer),
        );
    }

    commands.insert_resource(SampleMap(assets));

    Ok(())
}

#[derive(Resource)]
pub struct SampleMap(HashMap<String, SamplesBuffer<f32>>);

#[derive(Component)]
pub struct BasicRodioSink(Sink);

#[derive(Component)]
pub struct SpatialRodioSink(SpatialSink);

fn handle_sample_event(
    trigger: Trigger<AudioEvent>,
    context: Res<RodioStreamHandle>,
    samples: Res<SampleMap>,
    mut commands: Commands,
) -> Result {
    let sample = samples
        .0
        .get(trigger.sample)
        .cloned()
        .ok_or_else(|| format!("queued unknown sample {}", trigger.sample))?;

    // This makes both engines sound the same in terms of volume.
    let volume = firewheel::Volume::Linear(trigger.volume).amp();

    let mut new_sound = match trigger.position {
        Some(position) => {
            // here, we massage the distance so this sounds equivalent to firewheel
            let real_distance = position.length();
            let modified_distance = (10f32.powf(0.03 * real_distance)).sqrt();
            let direction = position.normalize_or_zero();
            let modified_emitter_pos = direction * modified_distance * 2.0;

            let sink = SpatialSink::try_new(
                &context.0,
                [modified_emitter_pos.x, modified_emitter_pos.y, 0.0],
                [-2.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
            )?;
            sink.set_volume(volume);
            sink.set_speed(trigger.speed);

            if trigger.looping {
                sink.append(sample.repeat_infinite());
            } else {
                sink.append(sample);
            }

            commands.spawn(SpatialRodioSink(sink))
        }
        None => {
            let sink = Sink::try_new(&context.0)?;
            sink.set_volume(volume);
            sink.set_speed(trigger.speed);

            if trigger.looping {
                sink.append(sample.repeat_infinite());
            } else {
                sink.append(sample);
            }

            commands.spawn(BasicRodioSink(sink))
        }
    };

    if let Some(name) = trigger.name {
        new_sound.insert(Name::new(name));
    }

    Ok(())
}

fn monitor_sinks(
    basic_sinks: Query<(Entity, &BasicRodioSink)>,
    spatial_sinks: Query<(Entity, &SpatialRodioSink)>,
    mut commands: Commands,
) {
    for (entity, sink) in &basic_sinks {
        if sink.0.empty() {
            commands.entity(entity).despawn();
        }
    }

    for (entity, sink) in &spatial_sinks {
        if sink.0.empty() {
            commands.entity(entity).despawn();
        }
    }
}

fn apply_fades(
    mut basic_sinks: Query<(Entity, &BasicRodioSink, &mut VolumeFade), Without<SpatialRodioSink>>,
    mut spatial_sinks: Query<(Entity, &SpatialRodioSink, &mut VolumeFade), Without<BasicRodioSink>>,
    mut commands: Commands,
    time: Res<Time>,
) -> Result {
    let delta = time.delta();

    for (entity, sink, mut fade) in &mut basic_sinks {
        fade.timer.tick(delta);
        let elapsed = fade.timer.elapsed_secs() / fade.timer.duration().as_secs_f32();

        // again, this just ensures both engines sound roughly the same
        let volume =
            firewheel::Volume::Linear(fade.event.start.lerp(fade.event.end, elapsed)).amp();

        sink.0.set_volume(volume);

        if fade.timer.finished() {
            commands.entity(entity).remove::<VolumeFade>();
        }
    }

    for (entity, sink, mut fade) in &mut spatial_sinks {
        fade.timer.tick(delta);
        let elapsed = fade.timer.elapsed_secs() / fade.timer.duration().as_secs_f32();

        let volume =
            firewheel::Volume::Linear(fade.event.start.lerp(fade.event.end, elapsed)).amp();

        sink.0.set_volume(volume);

        if fade.timer.finished() {
            commands.entity(entity).remove::<VolumeFade>();
        }
    }

    Ok(())
}
