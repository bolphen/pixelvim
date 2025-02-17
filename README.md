# <img width="48" height="48" align="top" src="https://raw.githubusercontent.com/bolphen/pixelvim/release/assets/icon.png"> pixelvim
`pixelvim` is a pixel editor inspired by the `vim` text editor, with an
emphasis on keyboard interaction. It also aims to be feature-rich,
customizable, and extendable via user scripts.

[Try it in your browser](https://bolphen.github.io/pixelvim/)

![screencast](https://github.com/user-attachments/assets/b8c26736-96ca-4220-975a-98116add6d66)

**Note. The editor is very much usable already, and there are data recovery à
la `vim` in case of crashes. However, the project is still in alpha, and you
are encouraged to back up your works.**

## Features
- `vi`-style remappable keyboard interaction and a command system
    - `5j` to move the selection 5 pixels down
    - `:map C-z :undo` maps `Ctrl-z` to undo
    - Commands can be chained: you can map a single key to `:select/all THEN
      cut THEN :layer/new/above THEN paste`
    - `:quantize 32 THEN :palette/build` first quantize the image into 32
      colors then rebuild the palette from it
    - Customizable via configuration file: put a `pixelvim.conf` in your
      `.config/` directory, or load on-the-fly using `:source new.conf`
    - Tab-completion and command history

- Advanced (and undo/redoable) pixel selection in `visual` mode, similar to
  `aseprite`
    - Create custom brush shape from selection using `:set brush/shape=%`

- Useful tools for creating animations. Draw while the animation is playing!
    - Either in "edit all" mode, so the changes apply to all frames;
    - Or in "live" mode, where each frame tracks its own change. This is
      particularly useful for quickly creating particle effects (fire, sparks,
      snowfall, ...)

- Rudimentary support for extensions via user scripts in lua
    - Run lua scripts that perform shader-like transformations on your image.
      Some example scripts are available, including "wave function collapse".
    - `:run script.lua` is a command so it can be mapped (or even chained) to a
      key

- Data recovery from swap file in case of crashes

## Build
Clone the repo
```
git clone https://github.com/bolphen/pixelvim
cd pixelvim
```
Then retrieve the submodules
```
git submodule update --init
```
Build and install to `.cargo/bin`
```
cargo install --path .
```

## Plans
I built this mainly for personal use, and I still have several ideas that I
wish to implement. Some design are not finalized. Also, the code is quite messy
and at places held up with glue. So do note that future updates may break your
configuration file.

## Implementation
`pixelvim` is built with the outstanding
[`miniquad`](https://github.com/not-fl3/miniquad). The lua extension is
made possible via [`mlua`](https://github.com/mlua-rs/mlua). Many inspirations
are taken from [`aseprite`](https://github.com/aseprite/aseprite) and
[`rx`](https://github.com/cloudhead/rx) (this is *not* a fork of `rx`).
