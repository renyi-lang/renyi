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
renyi run showcase/raycaster/raycaster.ry            # the first frame
renyi run showcase/raycaster/raycaster.ry demo       # the demo, every 20th frame as text
renyi run showcase/raycaster/raycaster.ry trace      # the demo, one summary line per frame
renyi run showcase/raycaster/raycaster.ry steps F5 R8 F22 S
renyi test showcase/raycaster/raycaster.ry           # every example and test of every module
```

A script move is a key and an optional frame count: `F` forward, `B` back,
`L` and `R` turn, `S` shoot, `Q` quit, and any other key waits. `F22` means
forward for 22 frames. A frame lasts 64 ms. Turning moves 2 angle steps per
millisecond, so `R8` is a quarter turn. Walking covers 3 cells a second.

## Modules

| File | Content |
|------|---------|
| `shapes.ry` | the display contract: `Colour`, `Shape`, the 320 by 200 screen |
| `input.ry` | the display contract: `Input`, `idle`, `repeated` |
| `fixed.ry` | fixed-point helpers and the sine and cosine table |
| `level.ry` | tiles, the level's text rows, parsing, doors |
| `raycast.ry` | camera rays, the DDA walk through the grid, camera-space projection |
| `player.ry` | turning, striding, movement with collision |
| `enemies.ry` | chasing, line of sight, bites, the hit scan |
| `game.ry` | `State`, `new_game`, `step`, `played` |
| `view.ry` | `render`: sky and floor, wall columns, sprites, the HUD |
| `text_view.ry` | `text_view`, which samples a frame into a character grid |
| `raycaster.ry` | `main`, which plays a script and prints frames |

## The display contract

A display needs three things: the types it draws, the input it sends, and
two calls.

```
type Colour            red, green, blue: Integer, 0 to 255
type Shape is one of
  Column(screen_x, top, bottom, colour)          a strip at column screen_x, rows top..bottom inclusive
  Rectangle(screen_x, screen_y, width, height, colour)
  Label(screen_x, screen_y, text, colour)
type Input             forward, backward, turn_left, turn_right, fire, quit: Boolean
                       elapsed_ms: Integer (the milliseconds since the previous frame)
```

The logical screen is 320 columns by 200 rows, with the origin at the top
left. A frame is a `List of Shape`, drawn in order, so later shapes cover
earlier ones. The text view gives a label character 4 columns by 5 rows, so a
font about 8 pixels high fits the HUD.

The fields are named `screen_x` and `screen_y`, not `x` and `y`, because
Renyi rejects single-letter names (`single-letter-identifier`).

A real-time display runs this loop:

```
let mutable state be game.new_game(level.maze_rows)
every frame:
  change state to game.step(state: state, controls: <the keys held, elapsed_ms>)
  draw view.render(state)
stop when state.status is not Playing
```

## What the browser build replaces

The browser build replaces `raycaster.ry` and `text_view.ry`. It runs the
loop above in real time, with `std.screen` drawing the shapes on a canvas
and the keyboard filling `Input`. All the other modules stay the same.
