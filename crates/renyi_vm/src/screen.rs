//! The screen of `std.screen` (decision AW31): a program hands a frame of
//! shapes to the host through `screen.present` and gets the frame's input
//! back. The host is a trait handed in through `Options::screen`, as the
//! streams and the watch are. A host that can answer at once (the console
//! host of `renyi run`) returns the `Input` and the program continues as
//! from any primitive; one that cannot (a page, whose thread must return
//! to the browser to draw) answers `Later`: the run suspends inside the
//! call with its frames kept (`Interrupt::Suspended`) and the host
//! resumes it with the input (`Vm::resume`). The frame and the input
//! cross the boundary as plain Rust values; the native of `std.screen`
//! converts them. A `Strip` and a `Picture` name an image and `play`
//! names a sound (decision AW35); the names are the host's to resolve:
//! the page's to the files served with the program, the console host's
//! to a block of the shade's glyph and to no sound.

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::rc::Rc;

use crate::clock::Stopwatch;

/// The logical screen's width in columns.
pub const WIDTH: i64 = 320;

/// The logical screen's height in rows.
pub const HEIGHT: i64 = 200;

/// A colour as three channels from 0 to 255.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colour {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// One drawing primitive of a frame, drawn in order on the logical
/// screen of `WIDTH` by `HEIGHT` with the origin at the top left; the
/// variants of `std.screen.Shape`, field for field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A vertical strip at column `screen_x` from row `top` to row
    /// `bottom`, both included.
    Column {
        screen_x: i64,
        top: i64,
        bottom: i64,
        colour: Colour,
    },
    Rectangle {
        screen_x: i64,
        screen_y: i64,
        width: i64,
        height: i64,
        colour: Colour,
    },
    Label {
        screen_x: i64,
        screen_y: i64,
        text: String,
        colour: Colour,
    },
    /// One column of the named image, stretched from row `top` to row
    /// `bottom`, both included, its colours scaled by `shade` in
    /// percent; the column is taken modulo the image's width.
    Strip {
        screen_x: i64,
        top: i64,
        bottom: i64,
        image: String,
        column: i64,
        shade: i64,
    },
    /// The named image stretched to `width` by `height` at a place,
    /// its colours scaled by `shade` in percent.
    Picture {
        screen_x: i64,
        screen_y: i64,
        width: i64,
        height: i64,
        image: String,
        shade: i64,
    },
}

/// The keys held during one frame and the milliseconds since the previous
/// frame: the record `std.screen.Input`, field for field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub forward: bool,
    pub backward: bool,
    pub turn_left: bool,
    pub turn_right: bool,
    pub fire: bool,
    pub quit: bool,
    pub elapsed_ms: i64,
}

/// What a host answers to a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The frame's input, at once.
    Now(Input),
    /// The host must return to its own loop before it can answer: the
    /// run suspends at the call, and the host resumes it with the input.
    Later,
}

/// A host that draws the frames of a program.
pub trait Screen {
    /// Draw the frame and answer its input, now or later.
    fn present(&mut self, frame: Vec<Shape>) -> Answer;

    /// Whether the host ever answers `Later`. A run under such a host
    /// stays on the interpreter, where a suspended call can be resumed;
    /// a host that always answers now leaves the run its machine code.
    fn suspends(&self) -> bool {
        false
    }

    /// Play the named sound once; a host without sound ignores it.
    fn play(&mut self, _sound: &str) {}
}

/// A host shared between the VM and whoever drives it: the VM borrows it
/// for the time of each `present`.
pub type SharedScreen = Rc<RefCell<dyn Screen>>;

/// The characters from dark to bright that stand for a colour.
const RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// The keys of the console host, a letter each.
const KEYS: &str = "w forward, s backward, a left, d right, f fire, q quit";

/// The console host of `renyi run` (decision AW32): every frame drawn
/// as text, each cell of a grid of characters the brightness of the
/// colour under the cell's middle (a label written from its cell to the
/// right, a strip or a picture a block of its shade's glyph; a sound
/// is ignored), then a line read, whose letters are the keys held: `w`
/// forward, `s` backward, `a` turn left, `d` turn right, `f` fire, `q`
/// quit; the end of the input is `quit`; the elapsed time is the real
/// one since the previous frame. It answers at once, so a run keeps its
/// machine code.
pub struct ConsoleScreen {
    out: Box<dyn Write>,
    input: Box<dyn BufRead>,
    columns: usize,
    rows: usize,
    previous: Stopwatch,
}

impl ConsoleScreen {
    /// A host over the streams, drawing into a grid of `columns` by
    /// `rows` characters (each at least one, at most the screen's).
    pub fn new(
        out: Box<dyn Write>,
        input: Box<dyn BufRead>,
        columns: usize,
        rows: usize,
    ) -> ConsoleScreen {
        ConsoleScreen {
            out,
            input,
            columns: columns.clamp(1, WIDTH as usize),
            rows: rows.clamp(1, HEIGHT as usize),
            previous: Stopwatch::start(),
        }
    }

    /// The host over the process's own streams at the terminal's size:
    /// the variables `COLUMNS` and `LINES` when they are set, else 80 by
    /// 24, with one row kept for the prompt.
    pub fn standard() -> SharedScreen {
        let size = |name: &str, default: usize| {
            std::env::var(name)
                .ok()
                .and_then(|text| text.trim().parse::<usize>().ok())
                .filter(|&value| value > 0)
                .unwrap_or(default)
        };
        let columns = size("COLUMNS", 80);
        let rows = size("LINES", 24).saturating_sub(1).max(1);
        Rc::new(RefCell::new(ConsoleScreen::new(
            Box::new(std::io::stdout()),
            Box::new(std::io::BufReader::new(std::io::stdin())),
            columns,
            rows,
        )))
    }

    /// The frame as text: a grid of `columns` by `rows` characters, each
    /// the sample of the logical screen under the cell's middle, the
    /// rows joined by line breaks.
    pub fn text_of(frame: &[Shape], columns: usize, rows: usize) -> String {
        let columns = columns.clamp(1, WIDTH as usize);
        let rows = rows.clamp(1, HEIGHT as usize);
        let cell_width = WIDTH / columns as i64;
        let cell_height = HEIGHT / rows as i64;
        let mut grid = vec![' '; columns * rows];
        for shape in frame {
            match shape {
                Shape::Column {
                    screen_x,
                    top,
                    bottom,
                    colour,
                } => paint(
                    &mut grid,
                    columns,
                    cells_covering(*screen_x, *screen_x, cell_width, columns),
                    cells_covering(*top, *bottom, cell_height, rows),
                    glyph(*colour),
                ),
                Shape::Rectangle {
                    screen_x,
                    screen_y,
                    width,
                    height,
                    colour,
                } => paint(
                    &mut grid,
                    columns,
                    cells_covering(*screen_x, screen_x + width - 1, cell_width, columns),
                    cells_covering(*screen_y, screen_y + height - 1, cell_height, rows),
                    glyph(*colour),
                ),
                Shape::Label {
                    screen_x,
                    screen_y,
                    text,
                    ..
                } => {
                    let column = (screen_x / cell_width).max(0) as usize;
                    let row = (screen_y / cell_height).max(0) as usize;
                    if row >= rows {
                        continue;
                    }
                    for (place, letter) in text.chars().enumerate() {
                        if column + place >= columns {
                            break;
                        }
                        grid[row * columns + column + place] = letter;
                    }
                }
                Shape::Strip {
                    screen_x,
                    top,
                    bottom,
                    shade,
                    ..
                } => paint(
                    &mut grid,
                    columns,
                    cells_covering(*screen_x, *screen_x, cell_width, columns),
                    cells_covering(*top, *bottom, cell_height, rows),
                    shade_glyph(*shade),
                ),
                Shape::Picture {
                    screen_x,
                    screen_y,
                    width,
                    height,
                    shade,
                    ..
                } => paint(
                    &mut grid,
                    columns,
                    cells_covering(*screen_x, screen_x + width - 1, cell_width, columns),
                    cells_covering(*screen_y, screen_y + height - 1, cell_height, rows),
                    shade_glyph(*shade),
                ),
            }
        }
        grid.chunks(columns)
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The cells of the size whose sample point, the cell's middle, lies
/// from `start` to `stop`, as a range of cell indices below `limit`.
fn cells_covering(start: i64, stop: i64, size: i64, limit: usize) -> std::ops::Range<usize> {
    let middle = size / 2;
    if stop < middle {
        return 0..0;
    }
    let low = (start - middle + size - 1).max(0) / size;
    let high = ((stop - middle) / size).min(limit as i64 - 1);
    if high < low {
        return 0..0;
    }
    low as usize..(high + 1) as usize
}

fn paint(
    grid: &mut [char],
    width: usize,
    across: std::ops::Range<usize>,
    down: std::ops::Range<usize>,
    symbol: char,
) {
    for row in down {
        for column in across.clone() {
            grid[row * width + column] = symbol;
        }
    }
}

/// The character for the brightness of a colour, by the luma weights of
/// Rec. 601.
fn glyph(colour: Colour) -> char {
    let luma =
        (colour.red as usize * 299 + colour.green as usize * 587 + colour.blue as usize * 114)
            / 1000;
    RAMP[(luma * RAMP.len() / 256).min(RAMP.len() - 1)]
}

/// The character for a shade in percent: the grey of that brightness.
fn shade_glyph(shade: i64) -> char {
    let level = (shade.clamp(0, 100) * 255 / 100) as u8;
    glyph(Colour {
        red: level,
        green: level,
        blue: level,
    })
}

impl Screen for ConsoleScreen {
    fn present(&mut self, frame: Vec<Shape>) -> Answer {
        let text = ConsoleScreen::text_of(&frame, self.columns, self.rows);
        let _ = writeln!(self.out, "{text}");
        let _ = write!(self.out, "{KEYS}, enter for the next frame> ");
        let _ = self.out.flush();
        let mut line = String::new();
        let read = self.input.read_line(&mut line).unwrap_or(0);
        let elapsed_ms = self.previous.elapsed_ms() as i64;
        self.previous = Stopwatch::start();
        let mut input = Input {
            elapsed_ms,
            quit: read == 0,
            ..Input::default()
        };
        for letter in line.trim().chars() {
            match letter {
                'w' => input.forward = true,
                's' => input.backward = true,
                'a' => input.turn_left = true,
                'd' => input.turn_right = true,
                'f' => input.fire = true,
                'q' => input.quit = true,
                _ => {}
            }
        }
        Answer::Now(input)
    }
}
