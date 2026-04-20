use std::f64::consts::PI;

use crate::{components::orbit::Orbit, scenes::epoch::EphemerisTime};

pub const EARTH_MASS_KG: f64 = 5.972e24;
pub const EARTH_RADIUS_M: f64 = 6.371e6;
pub const G: f64 = 6.674e-11 / (EARTH_RADIUS_M * EARTH_RADIUS_M * EARTH_RADIUS_M) * EARTH_MASS_KG;

pub struct HohmannTransfer {
    /// ET when to execute burn 1
    pub departure_time: EphemerisTime,
    /// ET when craft enters target SOI
    pub arrival_time: EphemerisTime,
    /// km/s, prograde burn to enter transfer ellipse
    pub departure_delta_v: f64,
    /// km/s, burn to circularize at target
    pub capture_delta_v: f64,
    /// The ellipse connecting the two orbits
    pub transfer_orbit: Orbit,
}

pub fn plan_hohmann(
    craft_orbit: &Orbit,
    target_orbit: &Orbit,
    current_et: EphemerisTime,
    parent_mass: f64, // in earth masses
) -> HohmannTransfer {
    let mu = G * parent_mass;
    let (r1, r2) = (craft_orbit.semi_major_axis, target_orbit.semi_major_axis);

    let (a_transfer, transfer_time_seconds, transfer_period_years) =
        transfer_ellipse_geometry(r1, r2, mu);

    let (departure_delta_v, capture_delta_v) = transfer_delta_v(r1, r2, mu);

    let departure_time =
        compute_departure_time(craft_orbit, target_orbit, current_et, transfer_time_seconds);

    let transfer_orbit = compute_transfer_orbit(
        craft_orbit,
        a_transfer,
        r1,
        r2,
        transfer_period_years,
        departure_time,
    );

    HohmannTransfer {
        departure_time,
        arrival_time: departure_time + EphemerisTime::from_secs(transfer_time_seconds),
        departure_delta_v,
        capture_delta_v,
        transfer_orbit,
    }
}

/// Computes the semi-major axis, transfer time, and period of the transfer ellipse
fn transfer_ellipse_geometry(r1: f64, r2: f64, mu: f64) -> (f64, f64, f64) {
    let a_transfer = (r1 + r2) / 2.0;
    let transfer_time_seconds = PI * (a_transfer.powi(3) / mu).sqrt();
    let transfer_period_years = 2.0 * transfer_time_seconds / (365.0 * 24.0 * 3600.0);
    (a_transfer, transfer_time_seconds, transfer_period_years)
}

/// Computes the delta-v for departure and capture burns
fn transfer_delta_v(r1: f64, r2: f64, mu: f64) -> (f64, f64) {
    let v1 = (mu / r1).sqrt();
    let v2 = (mu / r2).sqrt();
    let v_transfer_periapsis = (2.0 * mu * r2 / (r1 * (r2 + r2))).sqrt();
    let v_transfer_apoapsis = (2.0 * mu * r1 / (r2 * (r1 + r2))).sqrt();
    (v_transfer_periapsis - v1, v2 - v_transfer_apoapsis)
}

/// Computes the ET at which to depart for the transfer window
fn compute_departure_time(
    craft_orbit: &Orbit,
    target_orbit: &Orbit,
    current_et: EphemerisTime,
    transfer_time_seconds: f64,
) -> EphemerisTime {
    let current_time_years = current_et.as_years();
    let target_angular_velocity = 2.0 * PI / (target_orbit.period * 365.0 * 24.0 * 3600.0);
    let synodic_period = (1.0 / (1.0 / craft_orbit.period - 1.0 / target_orbit.period)).abs();

    let target_travel_during_transfer = target_angular_velocity * transfer_time_seconds;

    // Sample departure times across one synodic period to find the best window
    let n_samples = 1000;
    let mut best_wait_seconds = f64::MAX;

    for i in 0..n_samples {
        let sample_wait_seconds =
            (i as f64 / n_samples as f64) * synodic_period * 365.0 * 24.0 * 3600.0;
        let sample_departure_years =
            current_time_years + sample_wait_seconds / (365.0 * 24.0 * 3600.0);

        let craft_true_anomaly_at_departure = true_anomaly_at(craft_orbit, sample_departure_years);
        let target_true_anomaly_at_departure =
            true_anomaly_at(target_orbit, sample_departure_years);

        let required_target_angle_at_arrival = craft_true_anomaly_at_departure + PI;
        let required_target_angle_at_departure =
            required_target_angle_at_arrival - target_travel_during_transfer;

        let phase_error = (required_target_angle_at_departure - target_true_anomaly_at_departure)
            .rem_euclid(2.0 * PI);

        // phase_error close to 0 or 2*PI means the window is near this sample
        let phase_error_normalized = if phase_error > PI {
            phase_error - 2.0 * PI
        } else {
            phase_error
        };

        if phase_error_normalized.abs() < (PI / n_samples as f64)
            && sample_wait_seconds < best_wait_seconds
        {
            best_wait_seconds = sample_wait_seconds;
        }
    }

    current_et + EphemerisTime::from_secs(best_wait_seconds)
}

/// Computes the transfer orbit given the departure time and geometry
fn compute_transfer_orbit(
    craft_orbit: &Orbit,
    a_transfer: f64,
    r1: f64,
    r2: f64,
    transfer_period_years: f64,
    departure_time: EphemerisTime,
) -> Orbit {
    let departure_time_years = departure_time.as_years();

    // Periapsis points toward craft position at departure
    let craft_pos_at_departure = craft_orbit.position_at(departure_time_years);
    let argument_of_periapsis = craft_pos_at_departure.y.atan2(craft_pos_at_departure.x);

    // Mean anomaly is 0 at departure, back-calculate to epoch
    let mean_anomaly_at_epoch =
        (-2.0 * PI * departure_time_years / transfer_period_years).rem_euclid(2.0 * PI);

    Orbit {
        semi_major_axis: a_transfer,
        eccentricity: (r2 - r1) / (r2 + r1),
        inclination: craft_orbit.inclination,
        longitude_of_ascending_node: craft_orbit.longitude_of_ascending_node,
        argument_of_periapsis,
        mean_anomaly_at_epoch,
        period: transfer_period_years,
    }
}

/// True anomaly of a body on an orbit at a given time in years
fn true_anomaly_at(orbit: &Orbit, time_years: f64) -> f64 {
    let mean_anomaly = orbit.mean_anomaly_at_epoch + 2.0 * PI * time_years / orbit.period;
    mean_to_true_anomaly(mean_anomaly, orbit.eccentricity)
}

pub fn mean_to_true_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
    // Solve Kepler's equation iteratively
    let mut eccentric_anomaly = mean_anomaly;
    for _ in 0..100 {
        eccentric_anomaly = mean_anomaly + eccentricity * eccentric_anomaly.sin();
    }
    // Convert eccentric anomaly to true anomaly
    2.0 * ((1.0 + eccentricity).sqrt() * (eccentric_anomaly / 2.0).sin())
        .atan2((1.0 - eccentricity).sqrt() * (eccentric_anomaly / 2.0).cos())
}

pub fn orbital_period(semi_major_axis: f64, parent_mass: f64) -> f64 {
    let mu = G * parent_mass;
    let period_seconds = 2.0 * PI * (semi_major_axis.powi(3) / mu).sqrt();
    period_seconds / (365.0 * 24.0 * 3600.0) // convert to years
}
