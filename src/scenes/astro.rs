use std::f64::consts::PI;

use crate::{components::orbit::Orbit, scenes::events::EphemerisTime};

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
    let mu = G * parent_mass; // standard gravitational parameter

    let r1 = craft_orbit.semi_major_axis; // earth radii
    let r2 = target_orbit.semi_major_axis; // earth radii

    // Semi-major axis of the transfer ellipse
    let a_transfer = (r1 + r2) / 2.0;

    // Transfer time is half the period of the transfer ellipse
    let transfer_time_seconds = std::f64::consts::PI * (a_transfer.powi(3) / mu).sqrt();
    let transfer_period_years = 2.0 * transfer_time_seconds / (365.0 * 24.0 * 3600.0);
    let transfer_time_et = (transfer_time_seconds * 1_000_000.0) as EphemerisTime; // to microseconds

    // Delta-vs
    let v1 = (mu / r1).sqrt(); // circular velocity at r1
    let v_transfer_periapsis = (2.0 * mu * r2 / (r1 * (r1 + r2))).sqrt(); // velocity at periapsis of transfer ellipse
    let v2 = (mu / r2).sqrt(); // circular velocity at r2
    let v_transfer_apoapsis = (2.0 * mu * r1 / (r2 * (r1 + r2))).sqrt(); // velocity at apoapsis of transfer ellipse

    let departure_delta_v = v_transfer_periapsis - v1;
    let capture_delta_v = v2 - v_transfer_apoapsis;

    // Phase angle the target needs to be at for the craft to arrive at the same time
    // Target travels this many radians during transfer
    let target_angular_velocity =
        2.0 * std::f64::consts::PI / (target_orbit.period * 365.0 * 24.0 * 3600.0);
    let required_phase_angle =
        std::f64::consts::PI - target_angular_velocity * transfer_time_seconds;

    // Current phase angle between craft and target
    // We need an initial departure time estimate to find the craft's position
    // Use current_et as first estimate, will be refined later
    let current_time_years = current_et as f64 / (365.0 * 24.0 * 3600.0 * 1_000_000.0);
    let craft_mean_anomaly =
        craft_orbit.mean_anomaly_at_epoch + 2.0 * PI * current_time_years / craft_orbit.period;
    let target_mean_anomaly =
        target_orbit.mean_anomaly_at_epoch + 2.0 * PI * current_time_years / target_orbit.period;
    let craft_true_anomaly = mean_to_true_anomaly(craft_mean_anomaly, craft_orbit.eccentricity);
    let target_true_anomaly = mean_to_true_anomaly(target_mean_anomaly, target_orbit.eccentricity);

    // The transfer apoapsis is opposite the periapsis, so the target must be at
    // craft_true_anomaly + PI at arrival (in the parent-centered frame)
    let required_target_angle_at_arrival = craft_true_anomaly + PI;

    // Target must travel from its current angle to required_target_angle_at_arrival
    // in exactly transfer_time_seconds
    let target_angular_velocity = -2.0 * PI / (target_orbit.period * 365.0 * 24.0 * 3600.0);

    // How far the target travels during transfer
    let target_travel_during_transfer = target_angular_velocity * transfer_time_seconds;

    // Required current target angle so it ends up at the right place
    let required_current_target_angle =
        required_target_angle_at_arrival - target_travel_during_transfer;

    // Phase difference between where target is and where it needs to be
    let current_phase_angle = target_true_anomaly;
    let phase_difference =
        (required_current_target_angle - current_phase_angle).rem_euclid(2.0 * PI);

    let synodic_period = (1.0 / (1.0 / craft_orbit.period - 1.0 / target_orbit.period)).abs();
    let wait_seconds = phase_difference / (2.0 * PI) * synodic_period * 365.0 * 24.0 * 3600.0;
    let departure_time = current_et + (wait_seconds * 1_000_000.0) as EphemerisTime;

    // Now compute the actual transfer argument of periapsis using the real departure time
    let departure_time_years = departure_time as f64 / (365.0 * 24.0 * 3600.0 * 1_000_000.0);
    let craft_pos_at_departure = craft_orbit.position_at(departure_time_years);
    let transfer_argument_of_periapsis = craft_pos_at_departure.y.atan2(craft_pos_at_departure.x);

    let transfer_mean_anomaly_at_epoch =
        (-2.0 * PI * departure_time_years / transfer_period_years).rem_euclid(2.0 * PI);

    HohmannTransfer {
        departure_time,
        arrival_time: departure_time + transfer_time_et,
        departure_delta_v,
        capture_delta_v,
        transfer_orbit: Orbit {
            semi_major_axis: a_transfer,
            eccentricity: (r2 - r1) / (r2 + r1),
            inclination: craft_orbit.inclination,
            longitude_of_ascending_node: craft_orbit.longitude_of_ascending_node,
            argument_of_periapsis: transfer_argument_of_periapsis,
            mean_anomaly_at_epoch: transfer_mean_anomaly_at_epoch,
            period: transfer_period_years,
        },
    }
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
