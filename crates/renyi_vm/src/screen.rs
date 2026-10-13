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
//! converts them.

use std::cell::RefCell;
use std::rc::Rc;

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
}

/// A host shared between the VM and whoever drives it: the VM borrows it
/// for the time of each `present`.
pub type SharedScreen = Rc<RefCell<dyn Screen>>;
