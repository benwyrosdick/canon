use bevy::prelude::*;

pub const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
pub const DRAG: f32 = 0.065;
pub const BLAST_RADIUS: f32 = 8.0;
pub const CANNON_RADIUS: f32 = 1.65;
pub const MAX_FLIGHT: f32 = 18.0;

#[derive(Clone, Copy)]
pub struct Ball {
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
}

impl Ball {
    pub fn step(&mut self, dt: f32, wind: Vec3) -> Vec3 {
        let old = self.position;
        // Exact integration of dv/dt = gravity + drag * (wind - velocity).
        let terminal = wind + GRAVITY / DRAG;
        // exp_m1 avoids cancellation in 1 - exp(-drag * dt) at small fixed steps.
        let change = -(-DRAG * dt).exp_m1();
        self.position += terminal * dt + (self.velocity - terminal) * (change / DRAG);
        self.velocity += (terminal - self.velocity) * change;
        self.age += dt;
        old
    }
}

pub fn segment_sphere(from: Vec3, to: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let offset = from - center;
    let d = to - from;
    let c = offset.length_squared() - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    let a = d.length_squared();
    if a < 1e-10 {
        return None;
    }
    let b = offset.dot(d);
    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) / a;
    (0.0..=1.0).contains(&t).then_some(t)
}

pub fn blast_damage(distance: f32) -> f32 {
    120.0 * (1.0 - distance / BLAST_RADIUS).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_is_consistent_across_timesteps_and_wind_moves_shots() {
        let initial = Ball {
            position: Vec3::Y * 20.0,
            velocity: Vec3::new(30.0, 25.0, 0.0),
            age: 0.0,
        };
        let wind = Vec3::Z * 10.0;
        let mut large = initial;
        large.step(2.0, wind);
        let mut small = initial;
        for _ in 0..240 {
            small.step(1.0 / 120.0, wind);
        }
        assert!(large.position.distance(small.position) < 0.005);
        assert!(large.velocity.distance(small.velocity) < 0.005);
        assert!(small.position.z > 1.0);
        assert!(small.velocity.y < initial.velocity.y);
    }

    #[test]
    fn sweep_catches_fast_direct_hit_and_rejects_near_miss() {
        let t = segment_sphere(Vec3::X * -100.0, Vec3::X * 100.0, Vec3::ZERO, 2.0).unwrap();
        assert!((t - 0.49).abs() < 0.0001);
        assert!(
            segment_sphere(
                Vec3::new(-100.0, 2.1, 0.0),
                Vec3::new(100.0, 2.1, 0.0),
                Vec3::ZERO,
                2.0
            )
            .is_none()
        );
        assert_eq!(
            segment_sphere(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, 2.0),
            Some(0.0)
        );
    }

    #[test]
    fn blast_damage_is_bounded_and_falls_off() {
        assert_eq!(blast_damage(0.0), 120.0);
        assert_eq!(blast_damage(4.0), 60.0);
        assert_eq!(blast_damage(8.0), 0.0);
        assert_eq!(blast_damage(100.0), 0.0);
    }
}
