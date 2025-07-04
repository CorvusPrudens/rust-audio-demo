use bevy::{platform::collections::HashMap, prelude::*};
use firewheel::{
    CpalConfig, FirewheelContext, Volume,
    channel_config::NonZeroChannelCount,
    collector::ArcGc,
    diff::{Diff, Notify},
    nodes::sampler::{PlaybackState, RepeatMode, SamplerConfig, SamplerNode, SequenceType},
    sample_resource::SampleResource,
    sampler_pool::{FxChain, SamplerPool, SpatialBasicChain, WorkerID},
};
use std::sync::Arc;
use walkdir::WalkDir;

use crate::AudioEvent;

pub struct FirewheelPlugin;

impl Plugin for FirewheelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(initialize_firewheel)
            .add_systems(PreStartup, load_samples)
            .add_systems(PostUpdate, (monitor_workers, update_firewheel).chain())
            .add_observer(handle_sample_event);
    }
}

#[derive(Resource)]
struct SpatialPool(SamplerPool<SpatialBasicChain>);

#[derive(Resource)]
struct BasicPool(SamplerPool<EmptyChain>);

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
        .insert_resource(BasicPool(basic))
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
    mut basic: ResMut<BasicPool>,
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
            volume: Volume::Linear(trigger.volume),
            repeat_mode,
        })),
        speed: trigger.speed as f64,
        playback: Notify::new(PlaybackState::Play { delay: None }),
        ..Default::default()
    };

    if let Some(position) = trigger.position {
        let worker = spatial
            .0
            .new_worker(&params, false, &mut context, |fx_chain_state, cx| {
                let baseline = fx_chain_state.fx_chain.spatial_basic;

                fx_chain_state.fx_chain.spatial_basic.offset =
                    Vec3::new(position.x, 0.0, position.y);

                let mut queue = Vec::new();
                baseline.diff(
                    &fx_chain_state.fx_chain.spatial_basic,
                    Default::default(),
                    &mut queue,
                );

                fx_chain_state.fx_chain.spatial_basic.diff(
                    &baseline,
                    Default::default(),
                    &mut cx.event_queue(fx_chain_state.node_ids[0]),
                );
            })?;

        commands.spawn(SpatialWorker(worker.worker_id));
    } else {
        let worker = basic.0.new_worker(&params, true, &mut context, |_, _| ())?;

        commands.spawn(BasicWorker(worker.worker_id));
    }

    Ok(())
}

#[derive(Component)]
struct SpatialWorker(WorkerID);

#[derive(Component)]
struct BasicWorker(WorkerID);

fn monitor_workers(
    spatial: Query<(Entity, &SpatialWorker)>,
    basic: Query<(Entity, &BasicWorker)>,

    mut spatial_pool: ResMut<SpatialPool>,
    mut basic_pool: ResMut<BasicPool>,
    mut context: NonSendMut<FirewheelContext>,

    mut commands: Commands,
) {
    for (entity, worker) in &spatial {
        if spatial_pool.0.stopped(worker.0, &context) {
            spatial_pool.0.stop(worker.0, &mut context);
            commands.entity(entity).despawn();
        }
    }

    for (entity, worker) in &basic {
        if basic_pool.0.stopped(worker.0, &context) {
            basic_pool.0.stop(worker.0, &mut context);
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Default)]
struct EmptyChain;

impl FxChain for EmptyChain {
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

        cx.connect(sampler_node_id, dst_node_id, &connections, true)
            .unwrap();

        vec![]
    }
}
