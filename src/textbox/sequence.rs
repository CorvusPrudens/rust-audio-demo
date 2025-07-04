use bevy::{prelude::*, sprite::Anchor, text::TextBounds};
use bevy_pretty_text::prelude::*;
use bevy_sequence::{
    fragment::{DataLeaf, event::InsertBeginDown},
    prelude::*,
};
use rand::Rng;
use std::{marker::PhantomData, time::Duration};

use crate::audio::AudioEvent;

pub fn sequence_plugin(app: &mut App) {
    app.insert_resource(Character {
        name: None,
        text_sound: "talk-low.wav",
    })
    .add_event::<FragmentEvent<AudioSequence>>()
    .add_systems(Startup, generate_triangle)
    .add_systems(
        Update,
        (
            textbox_handler,
            animate_triangle,
            tick_pauses,
            sequence_runner,
            update_name,
        )
            .chain(),
    )
    .add_observer(observe_typewriter);
}

fn observe_typewriter(
    _: Trigger<GlyphRevealed>,
    mut commands: Commands,
    character: Res<Character>,
) {
    let mut rng = rand::thread_rng();

    commands.trigger(AudioEvent {
        sample: character.text_sound,
        speed: rng.gen_range(0.95..1.05),
        volume: 0.5,
        ..Default::default()
    });
}

#[derive(Resource)]
struct TriangleMesh {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

impl TriangleMesh {
    pub fn mesh(&self) -> impl Bundle {
        (
            Mesh2d(self.mesh.clone()),
            MeshMaterial2d(self.material.clone()),
        )
    }
}

fn generate_triangle(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let triangle_size = 10.0;
    let mesh = meshes.add(Triangle2d::new(
        Vec2::new(-triangle_size, triangle_size / 2.0),
        Vec2::new(triangle_size, triangle_size / 2.0),
        Vec2::new(0.0, -triangle_size / 2.0),
    ));
    let material = materials.add(Color::WHITE);

    commands.insert_resource(TriangleMesh { mesh, material });
}

#[derive(Component)]
struct Triangle {
    timer: Timer,
    up: bool,
}

#[derive(Clone)]
pub enum AudioSequence {
    Pause(Duration),
    Text(String),
}

impl IntoFragment<AudioSequence> for &'static str {
    fn into_fragment(self, context: &Context<()>, commands: &mut Commands) -> FragmentId {
        let leaf = DataLeaf::new(AudioSequence::Text(self.into()));

        <_ as IntoFragment<AudioSequence>>::into_fragment(leaf, context, commands)
    }
}

impl IntoFragment<AudioSequence> for f32 {
    fn into_fragment(self, context: &Context<()>, commands: &mut Commands) -> FragmentId {
        let leaf = DataLeaf::new(AudioSequence::Pause(Duration::from_secs_f32(self)));

        <_ as IntoFragment<AudioSequence>>::into_fragment(leaf, context, commands)
    }
}

pub struct DynamicText<S, O, M> {
    system: S,
    marker: PhantomData<fn() -> (O, M)>,
}

pub fn dynamic<S, O, M>(system: S) -> DynamicText<S, O, M>
where
    S: IntoSystem<(), O, M>,
{
    DynamicText {
        system,
        marker: PhantomData,
    }
}

impl<S, O, M> IntoFragment<AudioSequence> for DynamicText<S, O, M>
where
    S: IntoSystem<(), String, M> + Send + 'static,
{
    fn into_fragment(self, _: &Context<()>, commands: &mut Commands) -> FragmentId {
        let system = commands.register_system(self.system);
        let id = commands
            .spawn(bevy_sequence::fragment::Leaf)
            .insert_begin_down(move |event, world| {
                let string = world.run_system(system).unwrap();

                world.send_event(FragmentEvent {
                    id: event.id,
                    data: AudioSequence::Text(string),
                });
            })
            .id();

        FragmentId::new(id)
    }
}

#[derive(Component)]
struct Textbox(FragmentEndEvent);

#[derive(Component)]
pub struct TextboxContainer;

#[derive(Component)]
struct TextboxName;

fn textbox_handler(
    textbox: Single<(Entity, &Textbox, Has<TypeWriter>)>,
    triangle: Query<Entity, With<Triangle>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut end_events: EventWriter<FragmentEndEvent>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    let (entity, textbox, has_typewriter) = textbox.into_inner();

    if has_typewriter {
        commands
            .entity(entity)
            .remove::<(TypeWriter, Reveal)>()
            .trigger(TypeWriterFinished);
    } else {
        end_events.write(textbox.0);
        commands.entity(entity).despawn();

        if let Ok(triangle) = triangle.single() {
            commands.entity(triangle).despawn();
        }

        commands.trigger(AudioEvent {
            sample: "click.ogg",
            volume: 0.9,
            ..Default::default()
        });
    }
}

#[derive(Component)]
struct SequencePause {
    timer: Timer,
    event: FragmentEndEvent,
}

fn tick_pauses(
    mut pauses: Query<(Entity, &mut SequencePause)>,
    mut end_events: EventWriter<FragmentEndEvent>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let delta = time.delta();
    for (entity, mut pause) in &mut pauses {
        if pause.timer.tick(delta).just_finished() {
            commands.entity(entity).despawn();
            end_events.write(pause.event);
        }
    }
}

#[derive(Resource)]
pub struct Character {
    pub name: Option<&'static str>,
    pub text_sound: &'static str,
}

pub trait CharacterFragment
where
    Self: Sized + IntoFragment<AudioSequence>,
{
    fn narrator(self) -> impl IntoFragment<AudioSequence> {
        self.on_start(|mut character: ResMut<Character>| {
            character.name = None;
            character.text_sound = "talk-low.wav";
        })
    }

    fn stranger(self) -> impl IntoFragment<AudioSequence> {
        self.on_start(|mut character: ResMut<Character>| {
            character.name = Some("Stranger");
            character.text_sound = "talk.wav";
        })
    }

    fn aster(self) -> impl IntoFragment<AudioSequence> {
        self.on_start(|mut character: ResMut<Character>| {
            character.name = Some("Aster");
            character.text_sound = "talk.wav";
        })
    }
}

impl<T> CharacterFragment for T where T: IntoFragment<AudioSequence> {}

pub fn despawn_textbox(container: Query<Entity, With<TextboxContainer>>, mut commands: Commands) {
    if let Ok(container) = container.single() {
        commands.entity(container).despawn();
    }
}

fn sequence_runner(
    mut start_events: EventReader<FragmentEvent<AudioSequence>>,
    container: Query<Entity, With<TextboxContainer>>,
    server: Res<AssetServer>,
    mut commands: Commands,
) -> Result {
    for event in start_events.read() {
        match &event.data {
            AudioSequence::Pause(pause) => {
                commands.spawn(SequencePause {
                    timer: Timer::new(*pause, TimerMode::Once),
                    event: event.end(),
                });

                if let Ok(container) = container.single() {
                    commands.entity(container).despawn();
                }
            }
            AudioSequence::Text(text) => {
                let textbox_size = Vec2::new(500.0, 150.0);

                let container = container.single().unwrap_or_else(|_| {
                    let container = commands
                        .spawn((
                            TextboxContainer,
                            Sprite {
                                image: server.load("textbox2.png"),
                                ..Default::default()
                            },
                            Transform {
                                scale: Vec3::splat(2.0),
                                ..Default::default()
                            },
                        ))
                        .id();

                    commands.spawn((
                        ChildOf(container),
                        TextboxName,
                        Anchor::BottomLeft,
                        Transform::from_xyz(-textbox_size.x / 2.0, textbox_size.y / 2.0, 0.0),
                    ));

                    container
                });

                let x_margin = 0.05;
                let x_inverse_margin = 1.0 - x_margin;
                let horizontal_bounds = textbox_size.x * x_inverse_margin;

                let y_margin = 0.1;
                let y_inverse_margin = 1.0 - y_margin;
                let vertical_bounds = textbox_size.y * y_inverse_margin;

                let anchor = Vec2::new(-horizontal_bounds / 2.0, vertical_bounds / 2.0);

                commands
                    .spawn((
                        Textbox(event.end()),
                        TypeWriter::new(35.),
                        PrettyTextParser::parse(text)?,
                        TextBounds::new_horizontal(textbox_size.x * 0.95),
                        Transform::from_translation(anchor.extend(0.0)),
                        TextFont {
                            font_size: 19.0,
                            ..Default::default()
                        },
                        Anchor::TopLeft,
                        ChildOf(container),
                    ))
                    .observe(
                        move |_: Trigger<TypeWriterFinished>,
                              mut commands: Commands,
                              triangle: Res<TriangleMesh>| {
                            commands.entity(container).with_child((
                                Triangle {
                                    timer: Timer::new(
                                        Duration::from_secs_f32(0.5),
                                        TimerMode::Repeating,
                                    ),
                                    up: false,
                                },
                                triangle.mesh(),
                                Transform::from_translation(Vec3::new(
                                    textbox_size.x * 0.45,
                                    -textbox_size.y * 0.40,
                                    1.0,
                                )),
                            ));
                        },
                    );
            }
        }
    }

    Ok(())
}

fn animate_triangle(mut triangle: Query<(&mut Transform, &mut Triangle)>, time: Res<Time>) {
    for (mut transform, mut triangle) in &mut triangle {
        let delta = time.delta();

        if triangle.timer.tick(delta).just_finished() {
            let offset = if triangle.up {
                Vec3::new(0.0, -5.0, 0.0)
            } else {
                Vec3::new(0.0, 5.0, 0.0)
            };

            triangle.up = !triangle.up;

            transform.translation += offset;
        }
    }
}

fn update_name(
    mut names: Query<(Entity, Option<&mut Text2d>), With<TextboxName>>,
    character: Res<Character>,
    mut commands: Commands,
) {
    for (name, text) in &mut names {
        match (character.name, text) {
            (Some(name_string), Some(mut text)) => {
                text.clear();
                *text = name_string.into();
            }
            (Some(name_string), None) => {
                commands.entity(name).insert(Text2d::new(name_string));
            }
            (None, _) => {
                commands
                    .entity(name)
                    .remove::<(Text2d, TextLayout, TextFont, TextColor, TextBounds)>();
            }
        }
    }
}
