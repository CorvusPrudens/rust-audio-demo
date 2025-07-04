use bevy::{platform::collections::HashMap, prelude::*};
use firewheel::{
    CpalConfig, FirewheelContext, Volume,
    channel_config::NonZeroChannelCount,
    collector::ArcGc,
    diff::{Diff, Notify},
    nodes::{
        sampler::{PlaybackState, RepeatMode, SamplerConfig, SamplerNode, SequenceType},
        volume::{VolumeNode, VolumeNodeConfig},
    },
    sample_resource::SampleResource,
    sampler_pool::{FxChain, SamplerPool, SpatialBasicChain, WorkerID},
};
use std::{sync::Arc, time::Duration};
use walkdir::WalkDir;

use crate::audio_events::{AudioEvent, VolumeFade};

pub struct FirewheelPlugin;

impl Plugin for FirewheelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(initialize_firewheel)
            .add_systems(PreStartup, load_samples)
            .add_systems(
                Last,
                (
                    monitor_workers,
                    apply_volume_fade,
                    apply_spatial_fade,
                    update_firewheel,
                )
                    .chain(),
            )
            .add_observer(handle_sample_event);
    }
}

#[derive(Resource)]
struct SpatialPool(SamplerPool<SpatialBasicChain>);

#[derive(Resource)]
struct VolumePool(SamplerPool<VolumeChain>);

/// Here we initialize the Firewheel audio engine.
fn initialize_firewheel(app: &mut App) {
    let config = firewheel::FirewheelConfig::default();
    let stream_config = CpalConfig {
        output: firewheel::CpalOutputConfig {
            desired_block_frames: None,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut context = FirewheelContext::new(config);
    context.start_stream(stream_config.clone()).unwrap();

    // With Firewheel, we prefer the _sampler pool_ approach.
    let spatial = SamplerPool::new(
        24,
        SamplerConfig::default(),
        // straight to the output
        context.graph_out_node_id(),
        NonZeroChannelCount::STEREO,
        &mut context,
    );

    let basic = SamplerPool::new(
        24,
        SamplerConfig::default(),
        // straight to the output
        context.graph_out_node_id(),
        NonZeroChannelCount::STEREO,
        &mut context,
    );

    app.insert_non_send_resource(context)
        .insert_resource(VolumePool(basic))
        .insert_resource(SpatialPool(spatial));
}

fn load_samples(mut commands: Commands, context: NonSend<FirewheelContext>) -> Result {
    let sample_rate = context.stream_info().unwrap().sample_rate;
    let assets_path = std::path::Path::new("assets");

    let mut assets = HashMap::default();
    for asset_entry in WalkDir::new(assets_path).into_iter().filter_map(|e| e.ok()) {
        let string_name: String = asset_entry
            .path()
            .strip_prefix(assets_path)
            .unwrap()
            .to_string_lossy()
            .into();

        let mut loader = symphonium::SymphoniumLoader::new();
        let Ok(source) = firewheel::load_audio_file(
            &mut loader,
            asset_entry.path(),
            sample_rate,
            Default::default(),
        ) else {
            continue;
        };

        let sample = ArcGc::new_unsized(|| Arc::new(source) as Arc<dyn SampleResource>);

        assets.insert(string_name, sample);
    }

    commands.insert_resource(SampleMap(assets));

    Ok(())
}

/// Synchronize state with the context via message passing.
fn update_firewheel(mut context: NonSendMut<FirewheelContext>) -> Result {
    context.update().map_err(|e| format!("{e:#?}"))?;

    Ok(())
}

#[derive(Resource)]
pub struct SampleMap(HashMap<String, ArcGc<dyn SampleResource>>);

fn handle_sample_event(
    trigger: Trigger<AudioEvent>,
    mut spatial: ResMut<SpatialPool>,
    mut basic: ResMut<VolumePool>,
    mut context: NonSendMut<FirewheelContext>,
    samples: Res<SampleMap>,
    mut commands: Commands,
) -> Result {
    let repeat_mode = if trigger.looping {
        RepeatMode::RepeatEndlessly
    } else {
        RepeatMode::PlayOnce
    };

    let sample = samples
        .0
        .get(trigger.sample)
        .cloned()
        .ok_or_else(|| format!("queued unknown sample {}", trigger.sample))?;

    let params = SamplerNode {
        sequence: Notify::new(Some(SequenceType::SingleSample {
            sample,
            volume: Volume::Linear(1.0),
            repeat_mode,
        })),
        speed: trigger.speed as f64,
        playback: Notify::new(PlaybackState::Play { delay: None }),
        ..Default::default()
    };

    let mut new_sound = match trigger.position {
        Some(position) => {
            let worker =
                spatial
                    .0
                    .new_worker(&params, false, &mut context, |fx_chain_state, cx| {
                        let baseline = fx_chain_state.fx_chain.spatial_basic;

                        fx_chain_state.fx_chain.spatial_basic.offset =
                            Vec3::new(position.x, 0.0, position.y);
                        fx_chain_state.fx_chain.spatial_basic.volume =
                            Volume::Linear(trigger.volume);

                        fx_chain_state.fx_chain.spatial_basic.diff(
                            &baseline,
                            Default::default(),
                            &mut cx.event_queue(fx_chain_state.node_ids[0]),
                        );
                    })?;

            commands.spawn(SpatialWorker {
                id: worker.worker_id,
                timer: Timer::new(Duration::from_millis(250), TimerMode::Once),
            })
        }
        None => {
            let worker =
                basic
                    .0
                    .new_worker(&params, true, &mut context, |fx_chain_state, cx| {
                        let baseline = fx_chain_state.fx_chain.volume;
                        fx_chain_state.fx_chain.volume.volume = Volume::Linear(trigger.volume);

                        fx_chain_state.fx_chain.volume.diff(
                            &baseline,
                            Default::default(),
                            &mut cx.event_queue(fx_chain_state.node_ids[0]),
                        );
                    })?;

            commands.spawn(VolumeWorker {
                id: worker.worker_id,
                timer: Timer::new(Duration::from_millis(250), TimerMode::Once),
            })
        }
    };

    if let Some(name) = trigger.name {
        new_sound.insert(Name::new(name));
    }

    Ok(())
}

#[derive(Component)]
struct SpatialWorker {
    id: WorkerID,
    timer: Timer,
}

#[derive(Component)]
struct VolumeWorker {
    id: WorkerID,
    timer: Timer,
}

fn monitor_workers(
    mut spatial: Query<(Entity, &mut SpatialWorker)>,
    mut basic: Query<(Entity, &mut VolumeWorker)>,

    mut spatial_pool: ResMut<SpatialPool>,
    mut basic_pool: ResMut<VolumePool>,
    mut context: NonSendMut<FirewheelContext>,

    time: Res<Time>,
    mut commands: Commands,
) {
    let delta = time.delta();

    for (entity, mut worker) in &mut spatial {
        // We allow each worker some time to flush its sequence to the audio graph.
        // This is handled much more robustly in `bevy_seedling`.
        if worker.timer.tick(delta).finished() && spatial_pool.0.stopped(worker.id, &context) {
            spatial_pool.0.stop(worker.id, &mut context);
            commands.entity(entity).despawn();
        }
    }

    for (entity, mut worker) in &mut basic {
        if worker.timer.tick(delta).finished() && basic_pool.0.stopped(worker.id, &context) {
            basic_pool.0.stop(worker.id, &mut context);
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Default)]
struct VolumeChain {
    volume: VolumeNode,
}

impl FxChain for VolumeChain {
    fn construct_and_connect(
        &mut self,
        sampler_node_id: firewheel::node::NodeID,
        sampler_num_channels: NonZeroChannelCount,
        dst_node_id: firewheel::node::NodeID,
        dst_num_channels: NonZeroChannelCount,
        cx: &mut FirewheelContext,
    ) -> Vec<firewheel::node::NodeID> {
        let connections = (0..sampler_num_channels
            .get()
            .get()
            .min(dst_num_channels.get().get()))
            .map(|i| (i, i))
            .collect::<Vec<_>>();

        let volume_node = cx.add_node(
            VolumeNode::default(),
            Some(VolumeNodeConfig {
                channels: sampler_num_channels,
                ..Default::default()
            }),
        );

        cx.connect(sampler_node_id, volume_node, &connections, true)
            .unwrap();

        cx.connect(volume_node, dst_node_id, &connections, true)
            .unwrap();

        vec![volume_node]
    }
}

fn apply_volume_fade(
    mut workers: Query<(Entity, &VolumeWorker, &mut VolumeFade)>,
    mut pool: ResMut<VolumePool>,
    mut context: NonSendMut<FirewheelContext>,
    mut commands: Commands,
    time: Res<Time>,
) -> Result {
    let delta = time.delta();
    for (entity, worker, mut fade) in &mut workers {
        fade.timer.tick(delta);
        let elapsed = fade.timer.elapsed_secs() / fade.timer.duration().as_secs_f32();

        let chain = pool.0.fx_chain_mut(worker.id).ok_or("invalid worker ID")?;

        let baseline = chain.fx_chain.volume;
        chain.fx_chain.volume.volume =
            Volume::Linear(fade.event.start.lerp(fade.event.end, elapsed));

        chain.fx_chain.volume.diff(
            &baseline,
            Default::default(),
            &mut context.event_queue(chain.node_ids[0]),
        );

        if fade.timer.finished() {
            commands.entity(entity).remove::<VolumeFade>();
        }
    }

    Ok(())
}

fn apply_spatial_fade(
    mut workers: Query<(Entity, &SpatialWorker, &mut VolumeFade)>,
    mut pool: ResMut<SpatialPool>,
    mut context: NonSendMut<FirewheelContext>,
    mut commands: Commands,
    time: Res<Time>,
) -> Result {
    let delta = time.delta();
    for (entity, worker, mut fade) in &mut workers {
        fade.timer.tick(delta);
        let elapsed = fade.timer.elapsed_secs() / fade.timer.duration().as_secs_f32();

        let chain = pool.0.fx_chain_mut(worker.id).ok_or("invalid worker ID")?;

        let baseline = chain.fx_chain.spatial_basic;
        chain.fx_chain.spatial_basic.volume =
            Volume::Linear(fade.event.start.lerp(fade.event.end, elapsed));

        chain.fx_chain.spatial_basic.diff(
            &baseline,
            Default::default(),
            &mut context.event_queue(chain.node_ids[0]),
        );

        if fade.timer.finished() {
            commands.entity(entity).remove::<VolumeFade>();
        }
    }

    Ok(())
}
