use std::f64::consts::PI;

use crate::astro::{lambert::TransferKind, porkchop::Cell, state::State};

#[allow(unused)]
#[derive(Debug)]
pub enum TransferObjective {
    /// minimize total delta-v
    MinFuel,
    /// minimize tof, subject to a max delta-v budget
    MinTime { max_dv: f64 },
    /// weighed combination, alpha * dv + (1 - alpha) * tof
    Balanced { dv_weight: f64, tof_weight: f64 },
}

impl TransferObjective {
    /// return the cost given dv and tof, if feasible
    pub fn cost(&self, dv: f64, tof: f64) -> Option<f64> {
        match self {
            TransferObjective::MinFuel => Some(dv),
            TransferObjective::MinTime { max_dv } => {
                if dv <= *max_dv {
                    Some(tof)
                } else {
                    None
                }
            }
            TransferObjective::Balanced {
                dv_weight,
                tof_weight,
            } => Some(*dv_weight * dv + tof_weight * tof),
        }
    }
}

pub struct SweepWindow {
    pub sweep: f64,
    pub tof_min: f64,
    pub tof_max: f64,
}

pub fn sweep_window(craft: &State, target: &State, mu: f64) -> Result<SweepWindow, String> {
    let transfer_a = (craft.semi_major_axis(mu) + target.semi_major_axis(mu)) / 2.0;
    let tof_guess = PI * (transfer_a.powi(3) / mu).sqrt();

    let craft_period = craft
        .period(mu)
        .ok_or("can't transfer from a hyperbolic orbit")?;
    let target_period = target
        .period(mu)
        .ok_or("can't transfer to a hyperbolic orbit")?;

    let synodic = 1.0 / (1.0 / craft_period - 1.0 / target_period).abs();

    Ok(SweepWindow {
        sweep: synodic.min(craft_period * 20.0),
        tof_min: tof_guess * 0.7,
        tof_max: tof_guess * 1.5,
    })
}

pub fn best_branch<F>(f: F) -> Option<Cell>
where
    F: Fn(TransferKind) -> Option<Cell>,
{
    [TransferKind::Short, TransferKind::Long]
        .into_iter()
        .filter_map(f)
        .min_by(|a, b| a.total.total_cmp(&b.total))
}
