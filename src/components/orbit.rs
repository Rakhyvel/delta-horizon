use std::f64::consts::PI;

use nalgebra_glm::{vec3, vec4, DVec3};

use crate::scenes::astro::mean_to_true_anomaly;

#[derive(Copy, Clone)]
pub struct Orbit {
    pub semi_major_axis: f64, // In earth radii
    pub eccentricity: f64,
    pub inclination: f64,
    pub longitude_of_ascending_node: f64,
    pub argument_of_periapsis: f64,
    pub mean_anomaly_at_epoch: f64,
    pub period: f64, // In years
}

impl Orbit {
    pub fn generate_orbit_vertices(&self, segments: i32) -> Vec<f32> {
        let semi_major_axis = self.semi_major_axis;
        let eccentricity = self.eccentricity;
        let inclination = self.inclination;
        let lan = self.longitude_of_ascending_node; // Ω
        let arg_periapsis = self.argument_of_periapsis; // ω

        let semi_minor_axis = semi_major_axis * (1.0 - eccentricity * eccentricity).sqrt();
        // Offset so that the focus (parent body) is at the origin, not the center of the ellipse
        let focus_offset = semi_major_axis * eccentricity;

        let mut vertices = Vec::with_capacity((segments as usize + 1) * 3);

        // Rotation matrices for the three orbital elements:
        // 1. Argument of periapsis (ω) — rotates in the orbital plane
        // 2. Inclination (i) — tilts the orbital plane
        // 3. Longitude of ascending node (Ω) — rotates around the reference (z) axis
        let rot_arg_periapsis = nalgebra_glm::rotation(arg_periapsis, &vec3(0.0, 0.0, 1.0));
        let rot_inclination = nalgebra_glm::rotation(inclination, &vec3(1.0, 0.0, 0.0));
        let rot_lan = nalgebra_glm::rotation(lan, &vec3(0.0, 0.0, 1.0));
        let rotation = rot_lan * rot_inclination * rot_arg_periapsis;

        for i in 0..=segments {
            let angle = (i as f64 * 2.0 * PI) / segments as f64;

            // Ellipse in the orbital plane, offset so focus is at origin
            let x = semi_major_axis * angle.cos() - focus_offset;
            let y = semi_minor_axis * angle.sin();
            let z = 0.0;

            // Apply orbital rotations
            let pos = rotation * vec4(x, y, z, 1.0);

            vertices.push(pos.x as f32);
            vertices.push(pos.y as f32);
            vertices.push(pos.z as f32);
        }

        vertices
    }

    pub fn position_at(&self, t: f64) -> DVec3 {
        assert!(self.period > 0.0);
        assert!(self.eccentricity != 1.0);

        let mean_anomaly = self.mean_anomaly_at_epoch + 2.0 * PI * t / self.period;
        let true_anomaly = mean_to_true_anomaly(mean_anomaly, self.eccentricity);

        let r = self.semi_major_axis * (1.0 - self.eccentricity * self.eccentricity)
            / (1.0 + self.eccentricity * true_anomaly.cos());

        // Position in orbital plane
        let x = r * true_anomaly.cos();
        let y = r * true_anomaly.sin();

        // Apply orbital rotations (same order as vertex generation)
        let rot_arg_periapsis =
            nalgebra_glm::rotation(self.argument_of_periapsis as f32, &vec3(0.0_f32, 0.0, 1.0));
        let rot_inclination =
            nalgebra_glm::rotation(self.inclination as f32, &vec3(1.0_f32, 0.0, 0.0));
        let rot_lan = nalgebra_glm::rotation(
            self.longitude_of_ascending_node as f32,
            &vec3(0.0_f32, 0.0, 1.0),
        );
        let rotation = rot_lan * rot_inclination * rot_arg_periapsis;

        let pos = rotation * vec4(x as f32, y as f32, 0.0_f32, 1.0);
        vec3(pos.x as f64, pos.y as f64, pos.z as f64)
    }
}
