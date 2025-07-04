use bevy::prelude::*;
use std::time::Duration;

pub fn audio_events_plugin(app: &mut App) {
    app.add_observer(observe_fade_event);
}

/// An event to queue playback.
///
/// An event-based approach allows us to write one
/// sequence for all backends. Since these are observer-based
/// events, there won't be any meaningful latency introduced.
#[derive(Debug, Event, Clone)]
pub struct AudioEvent {
    pub sample: &'static str,
    pub position: Option<Vec2>,
    pub speed: f32,
    pub volume: f32,
    pub looping: bool,
    pub name: Option<&'static str>,
}

impl Default for AudioEvent {
    fn default() -> Self {
        Self {
            sample: "",
            position: None,
            speed: 1.0,
            volume: 1.0,
            looping: false,
            name: None,
        }
    }
}

#[derive(Event, Debug, Clone)]
pub struct VolumeFadeEvent {
    /// The name of the sample handle to target.
    pub name: &'static str,
    pub start: f32,
    pub end: f32,
    pub seconds: f32,
}

impl Default for VolumeFadeEvent {
    fn default() -> Self {
        Self {
            name: "",
            start: 0.0,
            end: 1.0,
            seconds: 1.0,
        }
    }
}

#[derive(Debug, Component)]
pub struct VolumeFade {
    pub event: VolumeFadeEvent,
    pub timer: Timer,
}

fn observe_fade_event(
    trigger: Trigger<VolumeFadeEvent>,
    named_entities: Query<(Entity, &Name)>,
    mut commands: Commands,
) -> Result {
    let event_name = Name::new(trigger.name);

    for (entity, name) in &named_entities {
        if name == &event_name {
            commands.entity(entity).insert(VolumeFade {
                timer: Timer::new(Duration::from_secs_f32(trigger.seconds), TimerMode::Once),
                event: trigger.clone(),
            });

            return Ok(());
        }
    }

    Err(format!("failed to find matching audio handle for name \"{event_name}\"").into())
}
