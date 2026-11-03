//! Where the sun is: NOAA's solar position algorithm (the Solar Calculator's spreadsheet formulas,
//! after Meeus, "Astronomical Algorithms"), good to a hundredth of a degree for these centuries.
//! Days are counted from 2000-01-01 00:00 UTC, as the kit's `Clock` counts them.

/// The sun's elevation above the horizon and its azimuth (clockwise from north), degrees, for a place
/// (degrees, east and north positive) at a moment (days since 2000-01-01 00:00 UTC).
pub fn position(lon: f64, lat: f64, day: f64) -> (f64, f64) {
    let jd = 2451544.5 + day;
    let jc = (jd - 2451545.0) / 36525.0;
    let l0 = (280.46646 + jc * (36000.76983 + jc * 0.0003032)).rem_euclid(360.0);
    let m = 357.52911 + jc * (35999.05029 - 0.0001537 * jc);
    let e = 0.016708634 - jc * (0.000042037 + 0.0000001267 * jc);
    let mr = m.to_radians();
    let c = mr.sin() * (1.914602 - jc * (0.004817 + 0.000014 * jc)) + (2.0 * mr).sin() * (0.019993 - 0.000101 * jc) + (3.0 * mr).sin() * 0.000289;
    let true_long = l0 + c;
    let omega = (125.04 - 1934.136 * jc).to_radians();
    let app_long = (true_long - 0.00569 - 0.00478 * omega.sin()).to_radians();
    let eps0 = 23.0 + (26.0 + (21.448 - jc * (46.815 + jc * (0.00059 - jc * 0.001813))) / 60.0) / 60.0;
    let eps = (eps0 + 0.00256 * omega.cos()).to_radians();
    let decl = (eps.sin() * app_long.sin()).asin();
    let y = (eps / 2.0).tan().powi(2);
    let l0r = l0.to_radians();
    let eqt = 4.0
        * (y * (2.0 * l0r).sin() - 2.0 * e * mr.sin() + 4.0 * e * y * mr.sin() * (2.0 * l0r).cos()
            - 0.5 * y * y * (4.0 * l0r).sin()
            - 1.25 * e * e * (2.0 * mr).sin())
        .to_degrees();
    let minutes = day.rem_euclid(1.0) * 1440.0;
    let tst = (minutes + eqt + 4.0 * lon).rem_euclid(1440.0);
    let ha = (tst / 4.0 - 180.0).to_radians();
    let phi = lat.to_radians();
    let el = (phi.sin() * decl.sin() + phi.cos() * decl.cos() * ha.cos()).clamp(-1.0, 1.0).asin();
    let az = (-ha.sin()).atan2(decl.tan() * phi.cos() - phi.sin() * ha.cos()).to_degrees().rem_euclid(360.0);
    (el.to_degrees(), az)
}

/// The moment of sunset (the sun's upper limb on the horizon, refraction included: -0.833°) on the
/// UTC day that `day` falls in, days since 2000.
pub fn sunset(lon: f64, lat: f64, day: f64) -> f64 {
    let d0 = day.floor();
    // From the local solar noon (roughly) to twelve hours after it, by halves.
    let noon = d0 + (0.5 - lon / 360.0);
    let (mut a, mut b) = (noon, noon + 0.5);
    for _ in 0..50 {
        let mid = 0.5 * (a + b);
        if position(lon, lat, mid).0 > -0.833 {
            a = mid;
        } else {
            b = mid;
        }
    }
    0.5 * (a + b)
}

/// Days since 2000-01-01 00:00 UTC, now.
pub fn now_days() -> f64 {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    (s - 946_684_800.0) / 86_400.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sunset_at_ham_wall_in_november() {
        // 2026-11-03 is day 9803; sunset at Glastonbury that day is about 16:42 UTC (timeanddate.com).
        let s = sunset(-2.787, 51.153, 9803.3);
        let minutes = (s - 9803.0) * 1440.0;
        assert!((minutes - (16.0 * 60.0 + 42.0)).abs() < 4.0, "sunset at {minutes} min");
        let (_, az) = position(-2.787, 51.153, s);
        assert!((az - 245.0).abs() < 4.0, "azimuth {az}");
    }
}
