# Talan
## A Final Fantasy XIV crafting bot

### Overview
Talan is a crafting bot currently driven by a command line interface. Rather than trying to be a crafting
solver like [FFXIV Crafting Optimizer](https://ffxiv-beta.lokyst.net/#/simulator), It reads in FFXIV macros directly.

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
- Using NQ or HQ materials base on priority.
- Determine crafting prerequisites and adding them to the task queue.
- Allowing default macros to be assigned to difficulty tiers / progress requirements.
