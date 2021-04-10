This library aims to handle getting information about and communicate with
monitors on Windows. Currently only getting information is supported.

For the equivalent on Linux see [xrandr-rs][xrandr-crate]

## Reliability

In short, this is an "It Works on my Machine" library. I've tested this library
on one device, a Surface Book 2 running Windows 10.

Based on my dependencies' documentation, listing all monitors probably works at
least as far back as Windows 8.1 and possibly as far back as Windows 7.

The approach used in this library for getting EDIDs is based off of old forum
posts and StackOverflow answers and relies on undocumented registry keys.
Some people on the internet report it works for many displays.

Getting monitors intersecting a window involves parsing an opaque ID with a
Regex. I would be surprised if it works reliably.

[xrandr-crate]: https://crates.io/crates/xrandr
