use std::ops::{Add, AddAssign, Sub};

/// Microseconds after the save start epoch
/// Should allow for ~292,000 years future and past
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub struct EphemerisTime(i64);

const SECONDS_PER_YEAR: f64 = 365.0 * 24.0 * 3600.0;
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

impl AddAssign for EphemerisTime {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
