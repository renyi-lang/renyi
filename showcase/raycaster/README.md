# Raycaster

A Doom-style maze shooter written in Renyi, drawn with the art of
[Freedoom](https://github.com/freedoom/freedoom). It opens on a title
screen; fire starts the game. You walk through rooms of STARTAN panels,
brick, bronze, marble and circuitry under a scrolling sky; doors rise when
you walk into them and come down again behind you. The demons sleep until
they see you, shout, and come for you, walking in frames A to D seen from
eight rotations (three of them the art's images drawn mirrored), biting in
frames E to G, flinching when hit and dying in six frames to a corpse that
stays on the floor; every column of a demon is hidden where a wall stands
nearer. The shotgun sways as you walk, fires on a fresh pull of the
trigger with a muzzle flash, then pumps and returns before it can fire
again, and counts its shells. The status bar has the original's layout:
ammo, health with its percent sign, the arms panel, the face that glances
about, winces, grins at a kill and bleeds as health falls, armor, and the
ammo tallies. Messages and the title are set in the heads-up font, and
every event has its sound. You win by walking into the green exit door and
lose when your health reaches zero; either way fire takes you back to the
title.

Each frame is a pure function of the state and the input, and the program
uses no randomness, so every run of a script gives the same frames and the
same sounds: `step` lists the sounds of its frame in the state, and `play`
plays them after the frame is drawn. All the arithmetic is on integers in
fixed point: one map cell is 4096 units and a full turn is 4096 angle
steps; a cell is 128 texels wide and high, as a wall of the original game,
and the eye stands 41 texels above the floor. The sine table is computed
once, by a Taylor series in `Float`, because the library has no
trigonometry.

## Play it locally

From the repository root:

```
renyi run showcase/raycaster/raycaster.ry play       # at the terminal: w a s d f q, enter per frame
renyi run showcase/raycaster/raycaster.ry            # the first frame of play
renyi run showcase/raycaster/raycaster.ry demo       # the demo, every 20th frame as text
renyi run showcase/raycaster/raycaster.ry trace      # the demo, one summary line per frame
renyi run showcase/raycaster/raycaster.ry steps F5 R8 F30 S
renyi test showcase/raycaster/raycaster.ry           # every example and test of every module
```

`play` starts at the title and hands every frame to the host of
`std.screen`: at the terminal (`renyi run`) the frame is drawn as text at
the terminal's size and a line is read, whose letters are the keys held
(`w` forward, `s` back, `a` and `d` turn, `f` fire, `q` quit; the end of
the input quits), the elapsed time being the real one; in the browser the
page is the host and draws the art. Fire starts the game from the title,
and, from a second after the game has ended, takes you back to it; quit
ends the program. `demo`, `trace` and `steps` start in play, without the
title, and stop when the game ends.

A script move is a key and an optional frame count: `F` forward, `B` back,
`L` and `R` turn, `S` fire, `Q` quit, and any other key waits. `F30` means
forward for 30 frames. A frame lasts 64 ms. Turning moves 2 angle steps per
millisecond, so `R8` is a quarter turn. Walking covers 3 cells a second. A
door takes 520 ms to rise and stays up 3 seconds; the shotgun can fire
again 940 ms after a shot, and only on a fresh pull, so `S1 W15` fires
once.

## Modules

| File | Content |
|------|---------|
| `shapes.ry` | the screen's size as constants and `shaded`, over the types of `std.screen` |
| `input.ry` | `idle` and `repeated`, over the `Input` of `std.screen` |
| `fixed.ry` | fixed-point helpers and the sine and cosine table |
| `level.ry` | tiles (a texture each), the level's text rows, parsing, how far each door is open |
| `raycast.ry` | the camera, the DDA walk through the grid and past rising doors, camera-space projection |
| `player.ry` | turning, striding, movement with collision |
| `enemies.ry` | the demons: waking, chasing, bites, the pain and death timers, the hit scan |
| `weapon.ry` | the shotgun: the fresh pull, the shells, the frames of a shot and the flash |
| `game.ry` | `State`, `title`, `new_game`, `step` (the title, play, the doors, the end), `played` |
| `sprites.ry` | the demon's image: the frame from its state, the rotation from the angle, mirroring |
| `font.ry` | the heads-up font: a text laid out as a `Picture` per letter |
| `hud.ry` | the status bar: the numbers, the face, the arms, the tallies |
| `view.ry` | `render`: the title; sky, floor, wall strips, demon strips, the shotgun, the bar, the words |
| `text_view.ry` | `text_view`, which samples a frame into a character grid; a strip or a picture a block of its shade's glyph |
| `art.ry` | the images of `art/` with their sizes and anchors, and the sounds, written by `tools/freedoom_art.py` |
| `raycaster.ry` | `main`: `play` at the host's screen, or a script played and printed as text |

## The display contract

The display contract is the library module `std.screen` (decisions AW31
and AW35): the types the game draws with, the input it receives, and
two calls, `screen.present(frame)`, which hands a frame to the host and
returns the frame's input, and `screen.play(sound)`, which plays a
named sound once on a host that can.

```
type Colour            red, green, blue: Integer, 0 to 255
type Shape is one of
  Column(screen_x, top, bottom, colour)          a strip at column screen_x, rows top..bottom inclusive
  Rectangle(screen_x, screen_y, width, height, colour)
  Label(screen_x, screen_y, text, colour)
  Strip(screen_x, top, bottom, image, column, shade)   one column of a named image, stretched between
                                                       the rows; column modulo the image's width; shade in percent
  Picture(screen_x, screen_y, width, height, image, shade)   a named image stretched to a size
type Input             forward, backward, turn_left, turn_right, fire, quit: Boolean
                       elapsed_ms: Integer (the milliseconds since the previous frame)
```

An image or a sound is named by the stem of a file under `art/`; the
page in the browser draws and plays those files, and the console host
draws a strip or a picture as a block of its shade's glyph and ignores
a sound.

The logical screen is 320 columns by 200 rows (`screen.width()` and
`screen.height()`), with the origin at the top left. A frame is a `List of
Shape`, drawn in order, so later shapes cover earlier ones. The game
draws its words as pictures of the font's letters, not as labels, and the
bottom 32 rows are the status bar, so the view of the maze is 320 by 168.

The fields are named `screen_x` and `screen_y`, not `x` and `y`, because
Renyi rejects single-letter names (`single-letter-identifier`).

`play` in `raycaster.ry` is the real-time loop:

```
let mutable state be game.title(level.maze_rows)
repeat until state.status is Quit
  let controls be screen.present(view.render(state))
  let next be game.step(state: state, controls: controls)
  for each sound in next.sounds
    screen.play(sound)
  end
  ...                                         # print the score when play ends
  change state to next
end
```

## In the browser

The game's page of the site (`docs/game.md`, decisions AW33 and AW35;
the play page links to it) is a host of `std.screen`: the toolchain
built for WebAssembly compiles these sources in the browser, the same
`play` runs there, `present` suspends the
program between frames, and the page draws the shapes on a canvas and
fills `Input` from the keyboard (`W` and `S` or the arrows move, `A` and
`D` turn, `Space` fires, `Escape` quits) and plays the sounds. Nothing
in the game changes between the terminal and the browser.
`tools/wasm_check.js --script` drives the same loop under node with a
fire to leave the title, then the demo's moves, and CI checks that it
ends as the demo does.

## The art

The images and the sounds under `art/` come from
[Freedoom](https://github.com/freedoom/freedoom), which publishes them
under the 3-clause BSD license, kept there verbatim as `COPYING.adoc`
(decision AW35). `tools/freedoom_art.py` downloads the chosen files at a
pinned commit, writes `art/README.md` with what came from where, and
writes `art.ry`, which lists every image with its size in pixels and
its anchor and every sound by name:

```
python -I tools/freedoom_art.py showcase/raycaster/art --ry showcase/raycaster/art.ry
```
