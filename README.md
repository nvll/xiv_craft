# Talan
## A Final Fantasy XIV crafting bot
[![Talan crafting 6x Grade 3 Infusion of Strength](http://img.youtube.com/vi/--hmcNVyhaA/0.jpg)](https://www.youtube.com/watch?v=--hmcNVyhaA)

### Overview
Talan is a crafting bot designed for max level crafters. Rather than trying to be a crafting
solver like [FFXIV Crafting Optimizer](https://ffxiv-beta.lokyst.net/#/simulator), It reads in FFXIV macros directly,
but executes them using its own engine that has none of the limitations of the in-game macro system.

Special thanks to Clorifex of [GarlandTools](https://garlandtools.org) and Miu of [FFXIV Teamcraft](https://ffxivteamcraft.com)
for various bits of help along the way.

### Features
Talan is still in alpha but already has a fairly solid set of features
- It can craft any number of a given item as long as the materials are NQ.
- It crafts faster than FFXIV's own macro interface because it can optimize for the GCD timing
  and the amount of time its own processing takes.
- It needs no action keybinds, it operates entirely through the text interface.
- It can parse any variation of FFXIV macros (quoted, unquoted, with wait, without wait)
- It can change gearsets to allow chaining of commands and crafts.
- It can craft collectable items.
- It verifies item names via Garlandtools.

### Roadmap
Talan is still under active development with the following roadmap in mind:
- Verifying all abilities in macros are valid.
- Setting appropriate role actions if a macro requires them.
- Using NQ or HQ materials based on priority.
- Determine crafting prerequisites and adding them to the task queue.
- Allowing default macros to be assigned to difficulty tiers / progress requirements.
- Building a web interface for using the tool.

### Usage
Right now Talan is driven via a command line interface and is not distributed as a binary.

To use it you will first need to install Rust via the [installation instructions](https://www.rust-lang.org/en-US/install.html). If given the option, you need the beta/2018 channel. Building Talan is a standard rust affair of:

```
chris@macbook ~/src/talan (master) $ cargo build
    Finished dev [unoptimized + debuginfo] target(s) in 0.24s
chris@macbook ~/src/talan (master) $ ./target/debug/talan -h
Talan 0.5.0
Christopher Anderson <chris@nullcode.org>

USAGE:
    talan [FLAGS] [OPTIONS] <macro file> <item name>

FLAGS:
        --collectable    Item(s) will be crafted as collectable
    -h, --help           Prints help information
    -d                   Increase delay between actions and UI navigation. Recommended with higher latency or input lag.
                         [UNIMPLEMENTED]
    -V, --version        Prints version information

OPTIONS:
    -c <count>               Number of items to craft [default: 1]
    -g <gearset>             Gearset to use for this crafting task. [default: 0]
    -i <recipe_index>        For recipes which have multiple search results this offset is used to determine the
                             specific recipe to use. Offsets start at 0 for the first recipe in search results and
                             increment by one for each recipe down. [default: 0]

ARGS:
    <macro file>    Path to the file containing the XIV macros to use
    <item name>     Name of the item to craft
```
