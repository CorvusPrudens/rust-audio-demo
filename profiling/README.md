## Audio Profiling

Disclaimer: I am a contributor to Firewheel.

The plots in this directory provide a decent overview of the
difference in execution time between the two engines. These
were taken over the course of the whole demo, with timings supplied
by Core Audio's client time.

Firewheel seems to beat `rodio` by around 3-5 times in my testing, and in
certain circumstances it was over 7 times faster. Firewheel's
jitter is also quite a bit lower -- `rodio`'s standard deviation
over the demo is larger than Firewheel's entire range!
