//! `std.csv`: comma-separated values with quoted cells (RFC 4180).

use std::rc::Rc;

use super::{arg, crash, list, text, NativeFn};
use crate::value::{Native, Value};
use crate::vm::{Interrupt, Vm};

pub fn lookup(name: &str, head: Option<&str>) -> Option<NativeFn> {
    Some(match (head, name) {
        (Some("Row"), "get") => row_get,
        (Some("Row"), "cells") => row_cells,
        (_, "parse") => parse,
        (_, "parse_without_header") => parse_without_header,
        (_, "render") => render,
        _ => return None,
    })
}

/// A record: the line it starts on and its cells.
pub type Record = (i64, Vec<String>);

/// The records of a CSV text; an error names the line and what is wrong.
pub fn parse_records(text: &str) -> Result<Vec<Record>, (i64, String)> {
    let mut records = Vec::new();
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut line = 1;
    let mut record_line = 1;
    let mut quoted = false;
    let mut was_quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        cell.push('"');
                    } else {
                        quoted = false;
                    }
                }
                '\n' => {
                    line += 1;
                    cell.push(c);
                }
                other => cell.push(other),
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() && !was_quoted => {
                quoted = true;
                was_quoted = true;
            }
            ',' => {
                cells.push(std::mem::take(&mut cell));
                was_quoted = false;
            }
            '\r' => {}
            '\n' => {
                cells.push(std::mem::take(&mut cell));
                was_quoted = false;
                records.push((record_line, std::mem::take(&mut cells)));
                line += 1;
                record_line = line;
            }
            other => cell.push(other),
        }
    }
    if quoted {
        return Err((record_line, "a quoted cell is not closed".to_string()));
    }
    if !cell.is_empty() || !cells.is_empty() {
        cells.push(cell);
        records.push((record_line, cells));
    }
    Ok(records)
}

fn csv_error(vm: &Vm, line: i64, detail: &str) -> Result<Value, Interrupt> {
    vm.fail_variant(
        "std.csv",
        "CsvError",
        "Malformed",
        vec![Value::integer(line), Value::text(detail)],
    )
}

fn parse(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let content = text(arg(args, 0))?;
    let records = match parse_records(content) {
        Ok(records) => records,
        Err((line, detail)) => return csv_error(vm, line, &detail),
    };
    let mut records = records.into_iter();
    let Some((_, header)) = records.next() else {
        return vm.fail_variant("std.csv", "CsvError", "MissingHeader", Vec::new());
    };
    let header = Rc::new(header);
    let mut rows = Vec::new();
    for (line, cells) in records {
        if cells.len() != header.len() {
            return csv_error(
                vm,
                line,
                &format!(
                    "the row has {} cells, the header {}",
                    cells.len(),
                    header.len()
                ),
            );
        }
        rows.push(Value::Native(Rc::new(Native::CsvRow {
            line,
            header: Rc::clone(&header),
            cells,
        })));
    }
    Ok(Value::list(rows))
}

fn parse_without_header(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let content = text(arg(args, 0))?;
    match parse_records(content) {
        Ok(records) => Ok(Value::list(
            records
                .into_iter()
                .map(|(_, cells)| Value::list(cells.into_iter().map(Value::from).collect()))
                .collect(),
        )),
        Err((line, detail)) => csv_error(vm, line, &detail),
    }
}

fn row(value: &Value) -> Result<(&Rc<Vec<String>>, &Vec<String>), Interrupt> {
    if let Value::Native(native) = value {
        if let Native::CsvRow { header, cells, .. } = &**native {
            return Ok((header, cells));
        }
    }
    Err(crash(format!(
        "expected a CSV Row, found {}",
        value.kind_name()
    )))
}

fn row_get(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let (header, cells) = row(arg(args, 0))?;
    let column = text(arg(args, 1))?;
    Ok(match header.iter().position(|name| name == column) {
        Some(index) if !cells[index].is_empty() => Value::text(&cells[index]),
        _ => Value::Nothing,
    })
}

fn row_cells(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let (_, cells) = row(arg(args, 0))?;
    Ok(Value::list(cells.iter().map(Value::text).collect()))
}

fn quote_cell(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

fn render(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let header = list(arg(args, 0))?;
    let rows = list(arg(args, 1))?;
    let mut lines = Vec::with_capacity(rows.len() + 1);
    let mut line = Vec::with_capacity(header.len());
    for cell in header.iter() {
        line.push(quote_cell(text(cell)?));
    }
    lines.push(line.join(","));
    for row in rows.iter() {
        let mut line = Vec::new();
        for cell in list(row)?.iter() {
            line.push(quote_cell(text(cell)?));
        }
        lines.push(line.join(","));
    }
    Ok(Value::text(format!("{}\n", lines.join("\n"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_cells_and_line_numbers() {
        let records = parse_records("a,b\n1,\"x,y\"\n\"multi\nline\",2\n").unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[1], (2, vec!["1".to_string(), "x,y".to_string()]));
        assert_eq!(records[2].0, 3);
        assert_eq!(records[2].1[0], "multi\nline");
        assert!(parse_records("a,\"open").is_err());
    }
}
