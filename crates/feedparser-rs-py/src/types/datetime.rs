use jiff::{Timestamp, tz::Offset};
use pyo3::prelude::*;

/// Convert `jiff::Timestamp` to Python `time.struct_time` in UTC for feedparser compatibility
pub fn datetime_to_struct_time(py: Python<'_>, ts: &Timestamp) -> PyResult<Py<PyAny>> {
    let time_module = py.import("time")?;
    let struct_time = time_module.getattr("struct_time")?;
    let dt = Offset::UTC.to_datetime(*ts);

    let tuple = (
        i32::from(dt.year()),
        i32::from(dt.month()),
        i32::from(dt.day()),
        i32::from(dt.hour()),
        i32::from(dt.minute()),
        i32::from(dt.second()),
        // Monday=0 in Python's time module
        i32::from(dt.weekday().to_monday_zero_offset()),
        i32::from(dt.day_of_year()),
        0i32, // tm_isdst (0 for UTC)
    );

    let result = struct_time.call1((tuple,))?;
    Ok(result.unbind())
}

pub fn optional_datetime_to_struct_time(
    py: Python<'_>,
    ts: &Option<Timestamp>,
) -> PyResult<Option<Py<PyAny>>> {
    match ts {
        Some(ts) => Ok(Some(datetime_to_struct_time(py, ts)?)),
        None => Ok(None),
    }
}

/// Formats a UTC timestamp as RFC 3339 with `+00:00` suffix and `AutoSi` fractional-second
/// precision (0, 3, 6, or 9 digits).
pub fn timestamp_to_rfc3339(ts: Timestamp) -> String {
    let ns = Offset::UTC.to_datetime(ts).subsec_nanosecond();
    let disp = ts.display_with_offset(Offset::UTC);
    if ns == 0 {
        format!("{disp}")
    } else if ns % 1_000_000 == 0 {
        format!("{disp:.3}")
    } else if ns % 1_000 == 0 {
        format!("{disp:.6}")
    } else {
        format!("{disp:.9}")
    }
}
