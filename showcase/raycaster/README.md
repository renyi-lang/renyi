# Raycaster

A maze shooter in the style of Wolfenstein 3D, written in Renyi. You walk
through a grid of stone, brick and wood walls, open doors by walking into
them, shoot the enemies that chase you, and win by walking into the green
exit. You lose when your health reaches zero.

Each frame is a pure function of the state and the input, and the program
uses no randomness, so every run of a script gives the same frames. All the
arithmetic is on integers in fixed point: one map cell is 4096 units and a
full turn is 4096 angle steps. The sine table is computed once, by a Taylor
series in `Float`, because the library has no trigonometry.

## Play it locally

From the repository root:

```
renyi run showcase/raycaster/raycaster.ry play       # at the terminal: w a s d f q, enter per frame
renyi run showcase/raycaster/raycaster.ry            # the first frame
renyi run showcase/raycaster/raycaster.ry demo       # the demo, every 20th frame as text
renyi run showcase/raycaster/raycaster.ry trace      # the demo, one summary line per frame
renyi run showcase/raycaster/raycaster.ry steps F5 R8 F22 S
renyi test showcase/raycaster/raycaster.ry           # every example and test of every module
```

`play` hands every frame to the host of `std.screen`: at the terminal
(`renyi run`) the frame is drawn as text at the terminal's size and a
line is read, whose letters are the keys held (`w` forward, `s` back, `a`
and `d` turn, `f` shoot, `q` quit; the end of the input quits), the
elapsed time being the real one; in the browser the page is the host.

A script move is a key and an optional frame count: `F` forward, `B` back,
`L` and `R` turn, `S` shoot, `Q` quit, and any other key waits. `F22` means
forward for 22 frames. A frame lasts 64 ms. Turning moves 2 angle steps per
millisecond, so `R8` is a quarter turn. Walking covers 3 cells a second.

## Modules

| File | Content |
|------|---------|
| `shapes.ry` | the screen's size as constants and `shaded`, over the types of `std.screen` |
| `input.ry` | `idle` and `repeated`, over the `Input` of `std.screen` |
| `fixed.ry` | fixed-point helpers and the sine and cosine table |
| `level.ry` | tiles, the level's text rows, parsing, doors |
| `raycast.ry` | camera rays, the DDA walk through the grid, camera-space projection |
| `player.ry` | turning, striding, movement with collision |
| `enemies.ry` | chasing, line of sight, bites, the hit scan |
| `game.ry` | `State`, `new_game`, `step`, `played` |
| `view.ry` | `render`: sky and floor, wall columns, sprites, the HUD |
| `text_view.ry` | `text_view`, which samples a frame into a character grid |
| `raycaster.ry` | `main`: `play` at the host's screen, or a script played and printed as text |

## The display contract

The display contract is the library module `std.screen` (decision AW31):
the types the game draws with, the input it receives, and one call,
`screen.present(frame)`, which hands a frame to the host and returns the
frame's input.

```
type Colour            red, green, blue: Integer, 0 to 255
type Shape is one of
  Column(screen_x, top, bottom, colour)          a strip at column screen_x, rows top..bottom inclusive
  Rectangle(screen_x, screen_y, width, height, colour)
  Label(screen_x, screen_y, text, colour)
type Input             forward, backward, turn_left, turn_right, fire, quit: Boolean
                       elapsed_ms: Integer (the milliseconds since the previous frame)
```

The logical screen is 320 columns by 200 rows (`screen.width()` and
`screen.height()`), with the origin at the top left. A frame is a `List of
Shape`, drawn in order, so later shapes cover earlier ones. The text view
gives a label character 4 columns by 5 rows, so a font about 8 pixels high
fits the HUD.

The fields are named `screen_x` and `screen_y`, not `x` and `y`, because
Renyi rejects single-letter names (`single-letter-identifier`).

`play` in `raycaster.ry` is the real-time loop:

```
let mutable state be game.new_game(level.maze_rows)
repeat until state.status is not Playing
  let controls be screen.present(view.render(state))
  change state to game.step(state: state, controls: controls)
end
```

## In the browser

The play page of the site (`docs/play.md`, decision AW33) is a host of
`std.screen`: the toolchain built for WebAssembly compiles these sources
in the browser, the same `play` runs there, `present` suspends the
program between frames, and the page draws the shapes on a canvas and
fills `Input` from the keyboard (`W` and `S` or the arrows move, `A` and
`D` turn, `Space` fires, `Escape` quits). Nothing in the game changes
between the terminal and the browser. `tools/wasm_check.js --script`
drives the same loop under node with the demo's moves, and CI checks
that it ends as the demo does.
