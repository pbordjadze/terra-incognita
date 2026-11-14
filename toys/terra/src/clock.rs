//! The clock's month, for the Earth's ground and ice.

/// The month now (0 = January), from the clock (a fixed month without one).
pub fn this_month() -> u32 {
    let secs = toy_kit::wall_clock() as u64;
    // civil from days (Howard Hinnant's algorithm)
    let z = (secs / 86_400) as i64 + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (m - 1) as u32
}
