//! `std.screen` (decision AW31): the frame of a program to the host that
//! draws it, the frame's input back, and the logical screen's size.

use super::{arg, crash, list, small, text};
use crate::extension::Native;
use crate::screen::{Answer, Colour, Input, Shape, HEIGHT, WIDTH};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

pub(crate) const NATIVES: &[Native] = &[
    Native::function("std.screen", "present", screen_present),
    Native::function("std.screen", "width", screen_width),
    Native::function("std.screen", "height", screen_height),
];

fn screen_width(_: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(WIDTH))
}

fn screen_height(_: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(HEIGHT))
}

/// The frame to the host. A host that answers now gives the input, which
/// goes back as the `Input` record. One that answers later suspends the
/// run: the boundary keeps what the call needs to be recorded, `execute`
/// keeps the frames, and `Vm::resume` pushes the input where this
/// primitive's answer would have landed. A suspension can be resumed
/// only from the run's own loop: inside a nested one (a module constant,
/// a comparison, a text conversion, a decode, a callback) or under
/// generated code, the call crashes instead.
fn screen_present(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let frame = frame_of(arg(args, 0))?;
    let Some(screen) = vm.screen.clone() else {
        return Err(crash(
            "the screen is not available here: the run has no host for `screen`",
        ));
    };
    let answer = screen.borrow_mut().present(frame);
    match answer {
        Answer::Now(input) => input_value(vm, input),
        Answer::Later if vm.can_suspend() => Err(Interrupt::Suspended),
        Answer::Later => Err(crash(
            "screen.present cannot wait here: the host answers later, and the call runs inside a module constant, a comparison, a text conversion, a decode or a callback, or on generated code",
        )),
    }
}

/// The frame as plain Rust: the list's variants by tag, their fields by
/// position as `std.screen` declares them; the checker has verified the
/// list is one of `Shape`.
fn frame_of(value: &Value) -> Result<Vec<Shape>, Interrupt> {
    let items = list(value)?;
    let mut shapes = Vec::with_capacity(items.len());
    for item in items.iter() {
        let Value::Variant(variant) = item.plain() else {
            return Err(crash("a frame holds something that is not a Shape"));
        };
        let fields = &variant.fields;
        let shape = match variant.tag {
            0 => Shape::Column {
                screen_x: small(&fields[0])?,
                top: small(&fields[1])?,
                bottom: small(&fields[2])?,
                colour: colour_of(&fields[3])?,
            },
            1 => Shape::Rectangle {
                screen_x: small(&fields[0])?,
                screen_y: small(&fields[1])?,
                width: small(&fields[2])?,
                height: small(&fields[3])?,
                colour: colour_of(&fields[4])?,
            },
            2 => Shape::Label {
                screen_x: small(&fields[0])?,
                screen_y: small(&fields[1])?,
                text: text(&fields[2])?.to_string(),
                colour: colour_of(&fields[3])?,
            },
            _ => return Err(crash("a frame holds something that is not a Shape")),
        };
        shapes.push(shape);
    }
    Ok(shapes)
}

/// A `Colour` record's channels, each clamped to 0 to 255.
fn colour_of(value: &Value) -> Result<Colour, Interrupt> {
    let Value::Record(record) = value.plain() else {
        return Err(crash("a shape's colour is not a Colour"));
    };
    let channel = |index: usize| -> Result<u8, Interrupt> {
        Ok(small(&record.fields[index])?.clamp(0, 255) as u8)
    };
    Ok(Colour {
        red: channel(0)?,
        green: channel(1)?,
        blue: channel(2)?,
    })
}

/// The host's input as the `Input` record of `std.screen`.
pub(crate) fn input_value(vm: &Vm, input: Input) -> Result<Value, Interrupt> {
    vm.library_record(
        "std.screen",
        "Input",
        vec![
            Value::Boolean(input.forward),
            Value::Boolean(input.backward),
            Value::Boolean(input.turn_left),
            Value::Boolean(input.turn_right),
            Value::Boolean(input.fire),
            Value::Boolean(input.quit),
            Value::integer(input.elapsed_ms),
        ],
    )
}
