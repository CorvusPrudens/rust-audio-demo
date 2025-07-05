# Real-time Rust audio demo

This repo provides a demonstration of real-time audio behavior for
the Firewheel and `rodio` audio engines. It takes the form of a
short interactive scene illustrated with sound and dialog.

Once compiled, you can select the engine at startup:

```bash
cargo run --release -- firewheel
cargo run --release -- rodio
```

## Notes

### Why use Bevy?

The goals of the audio demo do not conflict with writing it in Bevy.
Additionally, a number of Bevy's APIs make it easy to decouple the
core logic from the audio engine details.

The existing integrations for each engine are not used. The subset of
features required for this demo were integrated from scratch.

### Repeatability

The demo is structured as a core set of audio events and utilities
that are completely engine-agnostic (`src/audio`). The scripted sequences trigger
audio via these events, so the engine will not affect any timings.

Some pitches and timings are freely randomized, so the demo isn't
perfectly repeatable. If it turns out this is actually critical for
evaluation, we can easily adjust this approach.

There are some differences between the engines that are difficult to
compensate for. For example, `rodio` has individual ear positioning,
and the effect of distance on amplitude differs between the engines.
These differences are not especially important for the core evaluation,
so I wouldn't focus too much on small differences in volume or spatialization.

### Global volume

`rodio` 0.20 does not provide an easy way to assert a global volume without
interacting with all the sinks, so I opted _not_ to provide a way
to do that in the demo. To adjust the volume, you'll have to rely on
adjusting your system volume for now.

## Performance

The `profiling` directory provides a performance trace from my Macbook M3
laptop. The timings are taken from the Core Audio client execution time, so
they should be fairly accurate.

![Firewheel profiling results](https://github.com/CorvusPrudens/rust-audio-demo/blob/master/profiling/firewheel.png)
![rodio profiling results](https://github.com/CorvusPrudens/rust-audio-demo/blob/master/profiling/rodio.png)

Over the course of the demo, Firewheel is around 3x faster than `rodio`.
Keep in mind that, to my knowledge, the integration of `rodio` in this
crate is much more favorable than `bevy_audio`, while Firewheel's is less
favorable than `bevy_seedling`. In other words, it is easy to tune
Firewheel for even greater performance given a nice, higher-level API.
