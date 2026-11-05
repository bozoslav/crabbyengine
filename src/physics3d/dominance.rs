//! Rapier 0.36 treats a dominance-superior dynamic body as world-attached
//! (zero solver velocity). Restore its moving-surface normal velocity on the
//! lower body after solving. Rapier still handles positional correction and friction.
use super::*;

pub(super) fn transfer_velocity(physics: &mut Physics3D, dt: f32) {
    let world = &mut physics.world;
    for pair in world.narrow_phase.contact_pairs() {
        for manifold in pair.manifolds() {
            let data = &manifold.data;
            if data.relative_dominance == 0 || data.solver_contacts.is_empty() {
                continue;
            }
            let (Some(a), Some(b)) = (data.rigid_body1, data.rigid_body2) else {
                continue;
            };
            let (high, low, normal, collider) = if data.relative_dominance > 0 {
                (a, b, data.normal, pair.collider2)
            } else {
                (b, a, -data.normal, pair.collider1)
            };
            let (Some(higher), Some(lower)) = (world.bodies.get(high), world.bodies.get(low)) else {
                continue;
            };
            if !higher.is_dynamic() || !lower.is_dynamic() {
                continue;
            }
            let Some(shape) = world.colliders.get(collider) else {
                continue;
            };
            // Average the actual manifold points: a centered face contact must not
            // acquire torque merely from the arbitrary ordering of its corners.
            let mut point = rp::Vector::ZERO;
            let mut distance = 0.0;
            let mut count = 0;
            for contact in &manifold.points {
                if contact.dist <= world.integration_parameters.prediction_distance() {
                    let local = if data.relative_dominance > 0 {
                        contact.local_p2
                    } else {
                        contact.local_p1
                    };
                    point += *shape.position() * local;
                    distance += contact.dist;
                    count += 1;
                }
            }
            if count == 0 {
                continue;
            }
            point /= count as f32;
            distance /= count as f32;
            let closing = (higher.velocity_at_point(point) - lower.velocity_at_point(point)).dot(normal);
            // Speculative contacts must not push before the gap is crossed.
            let delta = closing - distance.max(0.0) / dt;
            if delta <= 0.0 {
                continue;
            }
            let arm = point - lower.center_of_mass();
            let torque = arm.cross(normal);
            let props = lower.mass_properties();
            let inverse_mass =
                normal.dot(props.effective_inv_mass * normal) + torque.dot(props.effective_world_inv_inertia * torque);
            if inverse_mass > f32::EPSILON {
                world.bodies.get_mut(low).expect("live body").apply_impulse_at_point(
                    normal * (delta / inverse_mass),
                    point,
                    true,
                );
            }
        }
    }
}
