use crate::astro::{
    departure::{best_branch, sweep_window, TransferObjective},
    epoch::EphemerisTime,
    lambert::lambert,
    porkchop::{Cell, Porkchop},
    state::State,
    units::{G, METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR},
};

#[derive(Clone, Copy)]
pub struct RendezvousPlan {
    pub transfer_state: State,
    pub rendezvous_state: State,
    pub transfer_dv: f64,
    pub brake_dv: f64,
    // TODO: Phasing maneuver
}

pub fn plan_rendezvous(
    craft_state: &State,
    target_state: &State,
    current_et: EphemerisTime,
    parent_mass: f64, // in earth masses
    objective: TransferObjective,
) -> Result<RendezvousPlan, String> {
    let mu = G * parent_mass;
    let w = sweep_window(craft_state, target_state, mu)?;

    let chop = Porkchop::compute(
        current_et,
        w.sweep,
        w.tof_min,
        w.tof_max,
        100,
        20,
        |et, tof| {
            let craft = craft_state.propagate(et, mu).ok()?;
            let target = target_state
                .propagate(et + EphemerisTime::from_years(tof), mu)
                .ok()?;

            best_branch(|k| {
                let (v1, v2) = lambert(craft.r, target.r, tof, mu, k)?;
                let depart_dv = v1 - craft.v;
                let brake = (target.v - v2).norm();
                Some(Cell {
                    total: depart_dv.norm() + brake,
                    depart_dv,
                    arrival_dv: brake,
                })
            })
        },
    );
    let (i, j, cell) = chop.best(&objective).ok_or("no feasible transfer found")?;
    let depart_et = chop.depart_at(i);
    let tof = chop.tof_at(j);

    let mut transfer_state = craft_state.propagate(depart_et, mu)?;
    transfer_state.v += cell.depart_dv;

    let arrival_et = depart_et + EphemerisTime::from_years(tof);
    let arrive = transfer_state.propagate(arrival_et, mu)?;
    let tgt = target_state.propagate(arrival_et, mu)?;
    let brake_dv = (tgt.v - arrive.v).norm();

    Ok(RendezvousPlan {
        transfer_state,
        transfer_dv: cell.depart_dv.norm() * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
        rendezvous_state: tgt,
        brake_dv: brake_dv * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
    })
}
