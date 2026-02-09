# Fractal SDF Ray Marching

This is an implementation of [Ray marching][raymarching] using [signed distance functions][sdf] (SDFs) to render [fractals][fractal]. It is implemented in Rust using [wgpu-rs](https://github.com/gfx-rs/wgpu).

[raymarching]: https://en.wikipedia.org/wiki/Ray_marching
[sdf]: https://en.wikipedia.org/wiki/Signed_distance_function
[fractal]: https://en.wikipedia.org/wiki/Fractal

## Building and Running

Install Rust and run one of these:
- `cargo build` to produce a binary at `target/debug/fractals[.exe]`.
- `cargo run` to directly build and run the binary

These will produce a _debug_ build. To produce a _release_ build, add `--release` to either of the commands.
The binary will be in `target/release/fractals[.exe]` instead.

The main difference between debug and release builds is not the performance (the heavy lifting is done on the GPU and the shaders are unaffected by the build mode), but whether the shaders are bundled or loaded from disk.
In debug mode, you can use the `r` key to reload [`fragment.wgsl`](./src/fragment.wgsl), which contains the ray marching code, SDFs and so on. This allows quickly iterating or changing of parameters without having to rerun the binary every time. In release mode, all shaders are bundled into the executable, so reloading does nothing, but you can distribute a single binary without worrying about accompanying files.

## Controls

These are the key and mouse bindings:

| binding                                     | action                                                                             |
| ------------------------------------------- | ---------------------------------------------------------------------------------- |
| `W`/`A`/`S`/`D`/`Q`/`E`                     | move forward, left, backward, right, down, up                                      |
| arrow keys                                  | turn left, right, up, down                                                         |
| left click                                  | capture mouse cursor                                                               |
| escape                                      | release mouse cursor                                                               |
| mouse move                                  | when captured, turn                                                                |
| scroll up/down                              | increase/decrease movement speed                                                   |
| scroll left/right or shift + scroll up/down | increase/decrease orbit speed (negative speed reverses direction)                  |
| `O`                                         | reset *o*rbiting speed to zero                                                     |
| `P`                                         | toggle *p*itch locking (vertical angle) to center of the model                     |
| `L`/shift + `L`                             | cycle forwards/backwards through yaw *l*ocking modes (horizontal angle, see below) |
| `+`/`-`                                     | increase/decrease number of fractal iterations                                     |
| `N`/`B`                                     | cycle through fractals (*n*ext / *b*ack)                                           |
| `>`/`<`                                     | increase/decrease render resolution                                                |
| `R`                                         | reload fragment shader                                                             |

The yaw locking feature has the following modes:
| mode     | effect                                                  |
| -------- | ------------------------------------------------------- |
| none     | no locking                                              |
| inwards  | always face the center of the fractal                   |
| right    | always face in the counter-clockwise orbiting direction |
| outwards | always face away from the fractal                       |
| left     | always face in the clockwise orbiting direction         |
