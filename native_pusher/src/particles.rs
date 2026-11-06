use glam::{Mat4, Quat, Vec3};
use rand::RngExt;

#[derive(Clone, Copy)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: [f32; 4],
    pub scale: f32,
    pub life: f32,
    pub max_life: f32,
    pub rotation: f32,
    pub rot_speed: f32,
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(512),
        }
    }

    /// Spawns an explosion of glittering celebration particles at a world position
    pub fn burst(&mut self, origin: Vec3, count: usize, is_fever: bool, color_tint: Option<[f32; 4]>) {
        let mut rng = rand::rng();

        for _ in 0..count {
            let vx = rng.random_range(-2.2..2.2);
            let vy = rng.random_range(3.2..7.8);
            let vz = rng.random_range(-1.2..2.5);

            let color = if let Some(tint) = color_tint {
                [tint[0], tint[1], tint[2], 1.0]
            } else if is_fever {
                // Rainbow fever colors
                match rng.random_range(0..5) {
                    0 => [1.0, 0.2, 0.5, 1.0], // Neon Pink
                    1 => [0.1, 0.9, 1.0, 1.0], // Neon Cyan
                    2 => [1.0, 0.9, 0.1, 1.0], // Bright Gold
                    3 => [0.3, 1.0, 0.4, 1.0], // Lime Green
                    _ => [0.8, 0.3, 1.0, 1.0], // Neon Purple
                }
            } else {
                // Golden spark & diamond colors
                match rng.random_range(0..4) {
                    0 => [1.0, 0.85, 0.2, 1.0], // 24K Gold
                    1 => [1.0, 0.98, 0.5, 1.0], // Sparkling Gold
                    2 => [0.2, 0.9, 1.0, 1.0],  // Diamond Cyan
                    _ => [1.0, 0.3, 0.3, 1.0],  // Ruby
                }
            };

            let life = rng.random_range(0.6..1.2);
            self.particles.push(Particle {
                position: origin + Vec3::new(rng.random_range(-0.4..0.4), rng.random_range(-0.1..0.2), rng.random_range(-0.2..0.2)),
                velocity: Vec3::new(vx, vy, vz),
                color,
                scale: rng.random_range(0.08..0.18),
                life,
                max_life: life,
                rotation: rng.random_range(0.0..std::f32::consts::TAU),
                rot_speed: rng.random_range(-10.0..10.0),
            });
        }
    }

    pub fn update(&mut self, dt: f32) {
        let gravity = Vec3::new(0.0, -13.5, 0.0);

        self.particles.retain_mut(|p| {
            p.life -= dt;
            if p.life <= 0.0 {
                return false;
            }

            p.velocity += gravity * dt;
            p.position += p.velocity * dt;
            p.rotation += p.rot_speed * dt;

            // Fade out
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            p.color[3] = alpha;

            true
        });
    }

    /// Converts active particles to transform matrices and colors for GPU instanced rendering
    pub fn get_instances(&self) -> Vec<(Mat4, [f32; 4])> {
        self.particles
            .iter()
            .map(|p| {
                let rot = Quat::from_rotation_z(p.rotation);
                let mat = Mat4::from_scale_rotation_translation(
                    Vec3::splat(p.scale),
                    rot,
                    p.position,
                );
                (mat, p.color)
            })
            .collect()
    }
}
