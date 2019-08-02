# XIVTools
## Ventures: A Venture Scheduling Automator
ventures is a simple bot that will reassign the ventures of specified retainers every sixty minutes. For more information check `-h`.

For best results, open the retainer window and then minimize the game while in windowed mode.

## Talan: A Crafting Bot
[![Talan (beta 2 candidate) crafting 5x Metal Gauntlets for the Crystalarium](http://i3.ytimg.com/vi/neSoWRJTPfE/maxresdefault.jpg)](https://www.youtube.com/watch?v=neSoWRJTPfE)

### Overview
Talan is a crafting bot designed for max level crafters. Rather than trying to be a crafting
solver like [FFXIV Crafting Optimizer](https://ffxiv-beta.lokyst.net/#/simulator), It reads in FFXIV macros directly,
but executes them using its own engine that has none of the limitations of the in-game macro system.

Special thanks to Clorifex of [GarlandTools](https://garlandtools.org) and Miu of [FFXIV Teamcraft](https://ffxivteamcraft.com)
for various bits of help along the way.

### Features
Talan is in its first beta, but already has a fairly solid set of features
- It has a working GUI that allows a user to queue up tasks made up of any number of crafts
  with different configurations. This includes items that can be crafted by multiple jobs,
  and collectable turn-ins.
- When not running in 'slow mode' it can overall craft 1/4th of a second *faster* per action than
  the in-game macro engine.
- It needs no action keybinds, it operates entirely through the text interface.
- It can parse any variation of FFXIV macros (quoted, unquoted, with wait, without wait)
- It can change gearsets to allow chaining of commands and crafts.
- It will use both NQ and HQ materials, prioritizing NQ.
- It uses XIVapi.com to lookup and configure crafts.

### Caveats / Known Issues
- Right now Talan assumes the basic keybinds for Confirm, Cancel, Up, Down, Left, Right, Backward,
  and Forward are default. If necessary, these can be changed in `xiv/src/ui.rs`.
- The XIV UI is fininky, so it's best to run the game in windowed mode and minimize it before starting a run
  of tasks to ensure you can't mistakenly alter the modality of the game's UI. Even moving the mouse over
  the window can interrupt the game's idea of which input method is being used.
- Talan will look in $the current working directory for a directory named `macros` to search for macros.
  If the `macros` directory doesn't exist then you'll experience a crash.

### Roadmap
Talan is still under active development with the following roadmap in mind:
- Configuring Role Actions if they are found in a macro. (possibly unnecessary in Shadowbringers)
- Automatically adding prereq crafts to the list.
- Using crafter food as necessary.

### Usage
Talan is largely controlled via the GUI. Run with -v, -vv, or -vvv for various amounts of debug
info.

To use it you will first need to install Rust via the [installation instructions](https://www.rust-lang.org/en-US/install.html). If given the option, you need the 2018 channel. Building Talan is a standard rust affair of:

```
Chris@DESKTOP ~/src/xivtools (beta) $ cargo build --release
   Compiling talan v0.5.0 (C:\Users\Chris\src\xivtools\talan)
    Finished release [optimized] target(s) in 4.01s
Chris@DESKTOP ~/src/xivtools (beta) $ ./target/release/talan.exe -h
Ventures

USAGE:
    talan.exe [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -s, --slow       Run with a longer delay between UI actions. Recommended for slower computers.
    -V, --version    Prints version information
    -v               Log level to use. Multiple can be used

SUBCOMMANDS:
    debug1
    debug2
    help      Prints this message or the help of the given subcommand(s)
```

debug1 and debug2 exist as subcommands to check if Talan can control the UI.
