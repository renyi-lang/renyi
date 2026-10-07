//! `std.time`: the clock, durations, calendar dates (records of the
//! library's `Date` type) and instants (milliseconds since the epoch, UTC).

use super::{arg, crash, duration, instant, now_millis, small, text, NativeFn};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

pub fn lookup(name: &str, head: Option<&str>) -> Option<NativeFn> {
    Some(match (head, name) {
        (None, "now") => time_now,
        (None, "today") => time_today,
        (Some("Integer"), "milliseconds") => time_milliseconds,
        (Some("Integer"), "seconds") => time_seconds,
        (Some("Integer"), "minutes") => time_minutes,
        (Some("Integer"), "hours") => time_hours,
        (Some("Duration"), "sleep") => time_sleep,
        (Some("Text"), "parse_date") => time_parse_date,
        (Some("Text"), "parse_instant") => time_parse_instant,
        (Some("Date"), "plus_days") => date_plus_days,
        (Some("Date"), "minus_days") => date_minus_days,
        (Some("Date"), "days_until") => date_days_until,
        (Some("Date"), "weekday") => date_weekday,
        (Some("Date"), "to_text") => date_to_text,
        (Some("Instant"), "plus") => instant_plus,
        (Some("Instant"), "minus") => instant_minus,
        (Some("Instant"), "elapsed_since") => instant_elapsed_since,
        (Some("Instant"), "date") => instant_date,
        (Some("Instant"), "to_text") => instant_to_text,
        _ => return None,
    })
}

// ---------------------------------------------------------- the calendar

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date of a day count since 1970-01-01.
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` of an instant, with milliseconds when not zero.
pub fn instant_text(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    let hour = rest / 3_600_000;
    let minute = rest % 3_600_000 / 60_000;
    let second = rest % 60_000 / 1000;
    let millis = rest % 1000;
    if millis == 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
    }
}

fn parse_date_parts(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split('-');
    let year = parts.next()?;
    let month = parts.next()?;
    let day = parts.next()?;
    if parts.next().is_some()
        || year.len() != 4
        || month.len() != 2
        || day.len() != 2
        || ![year, month, day]
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let (year, month, day) = (year.parse().ok()?, month.parse().ok()?, day.parse().ok()?);
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

/// Milliseconds of an ISO 8601 timestamp in UTC.
pub fn parse_instant_text(text: &str) -> Option<i64> {
    let text = text.strip_suffix('Z')?;
    let (date, clock) = text.split_once('T')?;
    let (year, month, day) = parse_date_parts(date)?;
    let (clock, millis) = match clock.split_once('.') {
        Some((clock, fraction)) => {
            if fraction.is_empty()
                || fraction.len() > 3
                || !fraction.chars().all(|c| c.is_ascii_digit())
            {
                return None;
            }
            (clock, format!("{fraction:0<3}").parse::<i64>().ok()?)
        }
        None => (clock, 0),
    };
    // two digits each, no sign: `13:45:00`
    let fields: Vec<&str> = clock.split(':').collect();
    if fields.len() != 3
        || fields
            .iter()
            .any(|field| field.len() != 2 || !field.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let hour: i64 = fields[0].parse().ok()?;
    let minute: i64 = fields[1].parse().ok()?;
    let second: i64 = fields[2].parse().ok()?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(
        days_from_civil(year, month, day) * 86_400_000
            + hour * 3_600_000
            + minute * 60_000
            + second * 1000
            + millis,
    )
}

impl Vm<'_> {
    pub fn date_value(&self, year: i64, month: i64, day: i64) -> Result<Value, Interrupt> {
        self.library_record(
            "std.time",
            "Date",
            vec![
                Value::integer(year),
                Value::integer(month),
                Value::integer(day),
            ],
        )
    }

    pub fn date_parts(&self, value: &Value) -> Result<(i64, i64, i64), Interrupt> {
        if let Value::Record(record) = value {
            if let [year, month, day] = record.fields.as_slice() {
                return Ok((small(year)?, small(month)?, small(day)?));
            }
        }
        Err(crash(format!(
            "expected a Date, found {}",
            value.kind_name()
        )))
    }

    pub fn date_days(&self, value: &Value) -> Result<i64, Interrupt> {
        let (year, month, day) = self.date_parts(value)?;
        Ok(days_from_civil(year, month, day))
    }

    pub fn date_from_days(&self, days: i64) -> Result<Value, Interrupt> {
        let (year, month, day) = civil_from_days(days);
        self.date_value(year, month, day)
    }
}

// --------------------------------------------------------------- the clock

fn time_now(_: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Instant(now_millis()))
}

fn time_today(vm: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    vm.date_from_days(now_millis().div_euclid(86_400_000))
}

fn time_sleep(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let ms = duration(arg(args, 0))?.max(0) as u64;
    std::thread::sleep(std::time::Duration::from_millis(ms));
    Ok(Value::Nothing)
}

fn scaled(args: &[Value], factor: i64) -> Result<Value, Interrupt> {
    Ok(Value::Duration(small(arg(args, 0))?.saturating_mul(factor)))
}

fn time_milliseconds(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    scaled(args, 1)
}

fn time_seconds(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    scaled(args, 1000)
}

fn time_minutes(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    scaled(args, 60_000)
}

fn time_hours(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    scaled(args, 3_600_000)
}

fn time_parse_date(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let input = text(arg(args, 0))?;
    match parse_date_parts(input) {
        Some((year, month, day)) => vm.date_value(year, month, day),
        None => vm.fail_record("std.time", "InvalidDate", vec![Value::text(input)]),
    }
}

fn time_parse_instant(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let input = text(arg(args, 0))?;
    match parse_instant_text(input) {
        Some(ms) => Ok(Value::Instant(ms)),
        None => vm.fail_record("std.time", "InvalidDate", vec![Value::text(input)]),
    }
}

// ------------------------------------------------------------------- dates

fn date_plus_days(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let days = vm.date_days(arg(args, 0))?;
    vm.date_from_days(days + small(arg(args, 1))?)
}

fn date_minus_days(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let days = vm.date_days(arg(args, 0))?;
    vm.date_from_days(days - small(arg(args, 1))?)
}

fn date_days_until(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let from = vm.date_days(arg(args, 0))?;
    let to = vm.date_days(arg(args, 1))?;
    Ok(Value::integer(to - from))
}

fn date_weekday(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let days = vm.date_days(arg(args, 0))?;
    // 1970-01-01 was a Thursday; Monday is tag 0
    let tag = (days + 3).rem_euclid(7) as usize;
    let ty = vm.library_type("std.time", "Weekday")?;
    Ok(Value::variant(ty, tag, Vec::new()))
}

fn date_to_text(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let (year, month, day) = vm.date_parts(arg(args, 0))?;
    Ok(Value::text(format!("{year:04}-{month:02}-{day:02}")))
}

// ---------------------------------------------------------------- instants

fn instant_plus(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Instant(
        instant(arg(args, 0))?.saturating_add(duration(arg(args, 1))?),
    ))
}

fn instant_minus(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Instant(
        instant(arg(args, 0))?.saturating_sub(duration(arg(args, 1))?),
    ))
}

fn instant_elapsed_since(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Duration(
        instant(arg(args, 0))?.saturating_sub(instant(arg(args, 1))?),
    ))
}

fn instant_date(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let ms = instant(arg(args, 0))?;
    vm.date_from_days(ms.div_euclid(86_400_000))
}

fn instant_to_text(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::text(instant_text(instant(arg(args, 0))?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_round_trip() {
        for (year, month, day) in [(1970, 1, 1), (2000, 2, 29), (2024, 5, 10), (1969, 12, 31)] {
            let days = days_from_civil(year, month, day);
            assert_eq!(civil_from_days(days), (year, month, day));
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2024, 5, 6) - days_from_civil(2024, 5, 3), 3);
    }

    #[test]
    fn instants_print_in_utc() {
        let ms = parse_instant_text("2024-05-01T13:45:00Z").unwrap();
        assert_eq!(instant_text(ms), "2024-05-01T13:45:00Z");
        assert_eq!(parse_instant_text("2024-13-01T00:00:00Z"), None);
        assert_eq!(parse_date_parts("2024-02-30"), None);
    }
}
