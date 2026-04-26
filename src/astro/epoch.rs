use std::ops::{Add, AddAssign, Div, Mul, Sub};

use chrono::{Datelike, Timelike};

use crate::astro::units::SECONDS_PER_YEAR;

/// Microseconds after the save start epoch
/// Should allow for ~292,000 years future and past
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub struct EphemerisTime(i64);

const ET_PER_SECOND: f64 = 1_000_000.0;
const ET_PER_YEAR: f64 = SECONDS_PER_YEAR * ET_PER_SECOND;

impl EphemerisTime {
    pub fn new(microsecs: i64) -> Self {
        Self(microsecs)
    }

    pub fn from_years(years: f64) -> Self {
        Self((years * ET_PER_YEAR) as i64)
    }

    pub fn from_secs(secs: f64) -> Self {
        Self((secs * ET_PER_SECOND) as i64)
    }

    pub fn as_years(self) -> f64 {
        (self.0 as f64) / ET_PER_YEAR
    }

    pub fn as_secs(self) -> f64 {
        (self.0 as f64) / ET_PER_SECOND
    }

    pub fn lerp(self, other: Self, t: f64) -> Self {
        let start = self.0;
        let end = other.0;
        Self(start + ((end - start) as f64 * t) as i64)
    }

    pub fn as_calendar(&self) -> String {
        let secs = self.0 / 1_000_000;
        let micros = self.0.rem_euclid(1_000_000) * 1000; // always positive

        let dt = chrono::DateTime::from_timestamp(secs, micros as u32).unwrap();

        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second()
        )
    }

    pub fn epoch() -> Self {
        let dt = chrono::NaiveDate::from_ymd_opt(1998, 12, 3)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        Self(dt.and_utc().timestamp() * 1_000_000)
    }
}

impl Add for EphemerisTime {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for EphemerisTime {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Mul<i64> for EphemerisTime {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self {
        Self(self.0 * rhs)
    }
}

impl Div<i64> for EphemerisTime {
    type Output = Self;
    fn div(self, rhs: i64) -> Self {
        Self(self.0 / rhs)
    }
}

impl AddAssign for EphemerisTime {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
