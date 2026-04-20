use std::f64::consts::PI;

use crate::{
    components::orbit::{Orbit, OrbitKind},
    scenes::epoch::EphemerisTime,
};

pub const EARTH_MASS_KG: f64 = 5.972e24;
pub const EARTH_RADIUS_M: f64 = 6.371e6;
pub const G: f64 = 6.674e-11 / (EARTH_RADIUS_M * EARTH_RADIUS_M * EARTH_RADIUS_M) * EARTH_MASS_KG;

// TODO: Rename to FlybyTransfer, not technically hohmann
pub struct HohmannTransfer {
    /// ET when to execute burn 1
    pub departure_time: EphemerisTime,
    /// ET when the craft enters the target SOI
    pub soi_entry_time: EphemerisTime,
    /// ET when craft is at target periapsis
    pub periapsis_time: EphemerisTime,
    /// km/s, prograde burn to enter transfer ellipse
    pub departure_delta_v: f64,
    /// The ellipse from craft to target body's SOI
    pub transfer_orbit: Orbit,
    /// The hyperbola around the target body
    pub flyby_orbit: Orbit,
    /// SOI, in earth radii, of the target body
    pub target_soi: f64,
}

pub fn plan_hohmann(
    craft_orbit: &Orbit,
    target_orbit: &Orbit,
    target_mass: f64,     // earth masses
    parent_mass: f64,     // earth masses
    flyby_periapsis: f64, // earth radii, from target body center
    current_et: EphemerisTime,
) -> HohmannTransfer {
    let mu_parent = G * parent_mass;
    let mu_target = G * target_mass;

    let (r1, r2) = (craft_orbit.semi_major_axis, target_orbit.semi_major_axis);

    println!("{} {}", mu_parent, parent_mass);

    // Transfer ellipse goes from craft orbit to target body's orbit
    // Same as a Hohmann but we don't circularize at r2
    let (a_transfer, transfer_time_seconds, transfer_period_years) =
        transfer_ellipse_geometry(r1, r2, mu_parent);

    let departure_delta_v = flyby_departure_delta_v(r1, r2, mu_parent);

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

    let target_soi = sphere_of_influence(target_orbit.semi_major_axis, target_mass, parent_mass);
    let soi_entry_time = compute_soi_entry_time(
        &transfer_orbit,
        target_orbit,
        target_soi,
        departure_time,
        transfer_time_seconds,
    );

    let flyby_orbit = compute_flyby_orbit(
        &transfer_orbit,
        target_orbit,
        mu_target,
        flyby_periapsis,
        departure_time,
        transfer_time_seconds,
    );

    let periapsis_time = departure_time
        + EphemerisTime::from_secs(transfer_time_seconds)
        + flyby_time_to_periapsis(&flyby_orbit, mu_target, flyby_periapsis);

    HohmannTransfer {
        departure_time,
        soi_entry_time,
        periapsis_time,
        departure_delta_v,
        transfer_orbit,
        flyby_orbit,
        target_soi,
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
fn flyby_departure_delta_v(r1: f64, r2: f64, mu: f64) -> f64 {
    let v1 = (mu / r1).sqrt();
    let v_transfer_periapsis = (2.0 * mu * r2 / (r1 * (r1 + r2))).sqrt();
    v_transfer_periapsis - v1
}

/// Computes the ET at which to depart for the transfer window
fn compute_departure_time(
    craft_orbit: &Orbit,
    target_orbit: &Orbit,
    current_et: EphemerisTime,
    transfer_time_seconds: f64,
) -> EphemerisTime {
    let current_time_years = current_et.as_years();
    let target_angular_velocity = 2.0 * PI / (target_orbit.period() * 365.0 * 24.0 * 3600.0);
    let synodic_period = (1.0 / (1.0 / craft_orbit.period() - 1.0 / target_orbit.period())).abs();

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

fn compute_soi_entry_time(
    transfer_orbit: &Orbit,
    target_orbit: &Orbit,
    target_soi: f64,
    departure_time: EphemerisTime,
    transfer_time_seconds: f64,
) -> EphemerisTime {
    let n_samples = 10_000;

    for i in 0..n_samples {
        let t = i as f64 / n_samples as f64;
        let sample_time = departure_time + EphemerisTime::from_secs(t * transfer_time_seconds);

        let craft_pos = transfer_orbit.position_at(sample_time);
        let target_pos = target_orbit.position_at(sample_time);
        let distance = (craft_pos - target_pos).norm();

        if distance < target_soi {
            return sample_time;
        }
    }

    // Fallback — shouldn't happen if SOI and transfer are computed correctly
    eprintln!("WARNING: SOI entry not found, falling back to arrival time");
    departure_time + EphemerisTime::from_secs(transfer_time_seconds)
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
    let craft_pos_at_departure = craft_orbit.position_at(departure_time);
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
        kind: OrbitKind::Periodic {
            period: transfer_period_years,
            mean_anomaly_at_epoch,
        },
    }
}

/// Computes the hyperbolic flyby orbit around the target body
fn compute_flyby_orbit(
    transfer_orbit: &Orbit,
    target_orbit: &Orbit,
    mu_target: f64,
    flyby_periapsis: f64,
    departure_time: EphemerisTime,
    transfer_time_seconds: f64,
) -> Orbit {
    let arrival_time = departure_time + EphemerisTime::from_secs(transfer_time_seconds);

    // Velocity of craft at transfer apoapsis (arrival at target)
    let r2 = target_orbit.semi_major_axis;
    let r1 = transfer_orbit.semi_major_axis * 2.0 - r2; // recover r1 from a_transfer
    let v_transfer_apoapsis = (2.0 * mu_target * r1 / (r2 * (r1 + r2))).sqrt();

    // Velocity of target body in its orbit
    let v_target = (G * mu_target / r2).sqrt();

    // Hyperbolic excess velocity — relative velocity at SOI entry
    let v_infinity = (v_transfer_apoapsis - v_target).abs();

    // Hyperbolic orbit parameters
    let (eccentricity, semi_major_axis) =
        hyperbolic_orbit_parameters(v_infinity, flyby_periapsis, mu_target);

    // Argument of periapsis points from target body toward craft arrival direction
    let target_pos_at_arrival = target_orbit.position_at(arrival_time);
    let argument_of_periapsis = target_pos_at_arrival.y.atan2(target_pos_at_arrival.x) + PI;

    Orbit {
        semi_major_axis,
        eccentricity,
        inclination: target_orbit.inclination,
        longitude_of_ascending_node: target_orbit.longitude_of_ascending_node,
        argument_of_periapsis,
        kind: OrbitKind::Hyperbolic {
            mu: mu_target,
            periapsis_time: arrival_time,
        },
    }
}

/// Returns (eccentricity, semi_major_axis) for a hyperbolic orbit
fn hyperbolic_orbit_parameters(v_infinity: f64, periapsis: f64, mu: f64) -> (f64, f64) {
    // Energy of hyperbolic orbit
    let specific_energy = v_infinity * v_infinity / 2.0;
    // Semi-major axis (negative for hyperbola)
    let a = -mu / (2.0 * specific_energy);
    // Eccentricity from periapsis and semi-major axis
    let e = 1.0 - periapsis / a;
    (e, a.abs())
}

/// Time from SOI entry to periapsis for a hyperbolic orbit
fn flyby_time_to_periapsis(flyby_orbit: &Orbit, mu: f64, periapsis: f64) -> EphemerisTime {
    // For a hyperbola, use the hyperbolic anomaly at SOI entry
    // Approximate SOI as being far enough that the asymptotic approximation holds
    let a = flyby_orbit.semi_major_axis;
    let e = flyby_orbit.eccentricity;
    // Hyperbolic mean motion
    let n = (mu / a.powi(3)).sqrt();
    // True anomaly at SOI — approximate as the asymptotic angle
    let true_anomaly_at_soi = -(1.0 / e).acos();
    // Convert to hyperbolic eccentric anomaly
    let f = ((e + true_anomaly_at_soi.cos()) / (1.0 + e * true_anomaly_at_soi.cos())).acosh();
    // Hyperbolic mean anomaly
    let m = e * f.sinh() - f;
    EphemerisTime::from_secs(m.abs() / n)
}

/// True anomaly of a body on an orbit at a given time in years
fn true_anomaly_at(orbit: &Orbit, time_years: f64) -> f64 {
    let mean_anomaly = orbit.mean_anomaly_at_epoch() + 2.0 * PI * time_years / orbit.period();
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

/// Computes true anomaly for a hyperbolic orbit at a given time offset from periapsis
pub fn hyperbolic_true_anomaly(
    dt_seconds: f64,
    semi_major_axis: f64,
    eccentricity: f64,
    mu: f64,
) -> f64 {
    // Hyperbolic mean motion
    let n = (mu / semi_major_axis.powi(3)).sqrt();
    // Hyperbolic mean anomaly
    let m = n * dt_seconds;

    // Solve hyperbolic Kepler's equation M = e*sinh(H) - H using Newton's method
    let mut h = m;
    for _ in 0..100 {
        let f = eccentricity * h.sinh() - h - m;
        let f_prime = eccentricity * h.cosh() - 1.0;
        h -= f / f_prime;
    }

    // Convert hyperbolic eccentric anomaly to true anomaly
    2.0 * (((eccentricity + 1.0) / (eccentricity - 1.0)).sqrt() * (h / 2.0).tanh()).atan()
}

pub fn orbital_period(semi_major_axis: f64, parent_mass: f64) -> f64 {
    let mu = G * parent_mass;
    let period_seconds = 2.0 * PI * (semi_major_axis.powi(3) / mu).sqrt();
    period_seconds / (365.0 * 24.0 * 3600.0) // convert to years
}

pub fn sphere_of_influence(orbital_radius: f64, body_mass: f64, parent_mass: f64) -> f64 {
    orbital_radius * (body_mass / parent_mass).powf(2.0 / 5.0)
}
