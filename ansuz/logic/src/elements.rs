//! Deliberate one-finger exercises. Recognition and response use delivered input;
//! neither pressure sensing nor a full fluid solver is implied.
use crate::lab::Contact;
use exact_game::*;
pub const PIXELS: usize = 240;
pub const SIZE: f32 = 0.32;
pub fn name(kind: u32) -> &'static str {
    match kind {
        1 => "Fire",
        2 => "Water",
        3 => "Earth",
        4 => "Air",
        _ => "Motion",
    }
}
#[derive(Clone, Copy, Default, Data)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub age: f32,
}
#[derive(Default, Resource)]
pub struct Elements {
    pub kind: u32,
    pub phase: u32,
    pub energy: f32,
    pub charge: f32,
    pub triggers: u32,
    pub object: Vec2,
    pub grabbed: bool,
    pub ticks: u32,
    pub particles: Vec<Particle>,
    /// Original pose recognizer, only for replaying model-1 Earth evidence.
    pub earth_legacy: bool,
    earth_peak: f32,
    earth_age: f32,
    earth_fired: bool,
    origin: Vec2,
    last: Vec2,
    offset: Vec2,
    still: f32,
    elapsed: f32,
    travel: f32,
    prepared: bool,
    winding: f32,
    spin: f32,
    bounds: Vec2,
    reduced: bool,
}
#[derive(Clone, Copy, Default)]
pub struct Pixel {
    pub position: Vec2,
    pub size: f32,
    pub color: [f32; 3],
    pub visible: bool,
}
fn unit(i: usize, salt: u32) -> f32 {
    let mut n = (i as u32)
        .wrapping_mul(747796405)
        .wrapping_add(salt.wrapping_mul(2891336453));
    n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
    ((n >> 22) ^ n) as f32 / u32::MAX as f32
}
impl Elements {
    pub fn reset(&mut self, kind: u32) {
        *self = Self {
            kind,
            bounds: Vec2::ONE,
            particles: if kind == 0 {
                Vec::new()
            } else {
                vec![Particle::default(); PIXELS]
            },
            ..Default::default()
        };
    }
    pub fn caption(&self) -> &'static str {
        match (self.kind, self.phase) {
            (1, 0) => "HOLD STILL",
            (1, 1) => "CHARGING",
            (1, 2) => "DRAW SLOWLY · THEN STRIKE",
            (1, _) => "IGNITED",
            (2, 0) => "GRAB · CIRCLE",
            (2, 1) => "KEEP CIRCLING",
            (2, _) => "VORTEX",
            (3, 0) if !self.earth_legacy => "SWEEP UP · THEN DOWN",
            (3, 1) if !self.earth_legacy => "GATHERING",
            (3, 2) if !self.earth_legacy => "TURN · DRIVE DOWN",
            (3, 3) if !self.earth_legacy => "DRIVE DOWN",
            (3, 0) => "BRACE",
            (3, 1) => "ROOTING",
            (3, 2) => "LIFT TO THE MARK",
            (3, 3) => "HOLD THE POSE",
            (3, _) => "RAISED",
            (4, 0) => "GRAB THE FAN",
            (4, _) => "FAN THE FIELD",
            _ => "MOTION",
        }
    }
    pub fn step(&mut self, c: &Contact, viewport: Vec2, dt: f32, reduced: bool) {
        if self.kind == 0 {
            return;
        }
        if self.particles.len() != PIXELS {
            self.reset(self.kind);
        }
        self.reduced = reduced;
        self.ticks = self.ticks.wrapping_add(1);
        self.bounds = Vec2::new((viewport.x / viewport.y.max(1.)).max(0.2), 1.);
        let p = (c.position - viewport * 0.5) * (2. / viewport.y.max(1.));
        self.elapsed += dt;
        self.energy = (self.energy - dt * if self.kind == 1 { 0.24 } else { 0.19 }).max(0.);
        self.spin *= math::exp(-2. * dt);
        if c.began {
            self.grabbed = (self.kind == 3 && !self.earth_legacy)
                || (p - self.object).abs().max_element() <= SIZE * 0.8;
            self.earth_peak = p.y;
            self.earth_age = 0.;
            self.earth_fired = false;
            self.origin = p;
            self.last = p;
            self.offset = self.object - p;
            self.still = 0.;
            self.elapsed = 0.;
            self.travel = 0.;
            self.prepared = false;
            self.winding = 0.;
            self.charge = 0.;
            if self.energy < 0.01 || (self.kind == 3 && !self.earth_legacy) {
                self.phase = 0;
            }
        }
        let delta = p - self.last;
        let moved = delta.length() > 0.0001;
        let speed = if moved {
            delta.length() / self.elapsed.max(dt)
        } else {
            0.
        };
        if self.grabbed && c.has_origin && (c.down || c.ended) {
            match self.kind {
                1 => {
                    if self.energy < 0.01 {
                        if self.phase < 2 {
                            if p.distance(self.origin) > 0.035 {
                                self.origin = p;
                                self.still = 0.;
                            }
                            self.still += dt;
                            self.charge = (self.still / 0.5).min(1.);
                            self.phase = if self.charge >= 1. { 2 } else { 1 };
                        } else if moved {
                            self.travel += delta.length();
                            if self.travel > 0.018 && speed < 0.85 {
                                self.prepared = true;
                            }
                            if self.prepared && self.travel > 0.14 && speed > 1.15 {
                                self.energy = 1.;
                                self.phase = 3;
                                self.triggers += 1;
                                self.charge = 0.;
                            }
                        }
                    }
                    self.object = (p + self.offset).clamp(
                        -self.bounds + Vec2::splat(SIZE),
                        self.bounds - Vec2::splat(SIZE),
                    );
                }
                2 => {
                    let a = self.last - self.object;
                    let b = p - self.object;
                    if moved && a.length() > 0.05 && b.length() > 0.05 && b.length() < 0.65 {
                        let angle = math::atan2(a.perp_dot(b), a.dot(b));
                        if angle.abs() < 0.65 {
                            if angle * self.winding < 0. {
                                self.winding *= 0.8;
                            }
                            self.winding += angle;
                            self.charge = (self.winding.abs() / 4.7).min(1.);
                            self.energy = (self.energy + angle.abs() * 0.18).min(1.);
                            self.spin =
                                (self.spin + angle / self.elapsed.max(dt) * 0.08).clamp(-3., 3.);
                            if self.phase < 2 && self.winding.abs() > 4.7 {
                                self.phase = 2;
                                self.triggers += 1;
                            }
                        }
                    }
                    if self.phase == 0 {
                        self.phase = 1;
                    }
                }
                3 if !self.earth_legacy => {
                    // A demonstrated sweep is a sequence, independent of screen position
                    // and lateral arc: gather upward, then commit with a downward return.
                    // A release/cancel cannot complete it, nor can a single-frame jump.
                    self.earth_age += dt;
                    if !self.earth_fired && c.down && self.earth_age <= 4. {
                        if self.phase < 2 && p.y > self.origin.y {
                            self.origin.y = p.y;
                            self.earth_peak = p.y;
                        }
                        self.earth_peak = self.earth_peak.min(p.y);
                        let rise = (self.origin.y - self.earth_peak).max(0.);
                        let returning = (p.y - self.earth_peak).max(0.);
                        self.charge = (rise / 0.8).min(1.);
                        if rise >= 0.4 && self.earth_age >= 0.12 {
                            self.phase = if returning > 0.08 { 3 } else { 2 };
                            if returning >= (rise * 0.55).max(0.22) && self.earth_age >= 0.25 {
                                self.earth_fired = true;
                                self.phase = 4;
                                self.energy = 1.;
                                self.charge = 0.;
                                self.triggers += 1;
                            }
                        } else {
                            self.phase = 1;
                        }
                    } else if !self.earth_fired {
                        self.charge = 0.;
                        self.phase = 0;
                    }
                }
                3 => {
                    if self.energy < 0.01 {
                        match self.phase {
                            0 | 1 => {
                                if p.distance(self.origin) > 0.035 {
                                    self.origin = p;
                                    self.still = 0.;
                                }
                                self.still += dt;
                                self.charge = (self.still / 0.55).min(1.);
                                self.phase = if self.charge >= 1. { 2 } else { 1 };
                            }
                            2 | 3 => {
                                let target = self.origin - Vec2::Y * 0.32;
                                if (p.x - self.origin.x).abs() > 0.13 {
                                    self.phase = 0;
                                    self.charge = 0.;
                                    self.still = 0.;
                                    self.origin = p;
                                } else if p.distance(target) < 0.075 {
                                    if self.phase != 3 {
                                        self.still = 0.;
                                    }
                                    self.phase = 3;
                                    self.still += dt;
                                    self.charge = (self.still / 0.3).min(1.);
                                    if self.still >= 0.3 {
                                        self.phase = 4;
                                        self.energy = 1.;
                                        self.triggers += 1;
                                    }
                                } else {
                                    self.phase = 2;
                                    self.still = 0.;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                4 => {
                    self.phase = 1;
                    self.object = (p + self.offset).clamp(
                        -self.bounds + Vec2::splat(SIZE * 0.6),
                        self.bounds - Vec2::splat(SIZE * 0.6),
                    );
                    self.energy = (self.energy + delta.length() * 0.7).min(1.);
                }
                _ => {}
            }
        }
        if moved {
            self.last = p;
            self.elapsed = 0.;
        }
        if !c.down || !c.has_origin {
            self.grabbed = false;
            self.charge = 0.;
            if self.energy < 0.01 {
                self.phase = 0;
            }
        }
        if self.energy <= 0.001 && !self.grabbed {
            self.phase = 0;
        }
        if self.kind == 1 && self.energy <= 0.001 && self.phase == 3 {
            self.phase = 0;
            self.still = 0.;
            self.origin = p;
        }
        if self.kind == 3 && self.energy <= 0.001 && self.phase == 4 {
            self.phase = 0;
            self.still = 0.;
            self.origin = p;
        }
        for i in 0..PIXELS {
            let home = Vec2::new((i % 12) as f32 - 5.5, (i / 12) as f32 - 5.5) * 0.025;
            let q = &mut self.particles[i];
            if self.kind == 2 && i < 144 {
                if self.energy < 0.005 {
                    q.position = self.object + home;
                    q.velocity = Vec2::ZERO;
                    continue;
                }
                let r = q.position - self.object;
                let influence = math::exp(-q.position.distance_squared(p) / 0.045);
                let tangent = Vec2::new(-r.y, r.x);
                let force = -r * (18. + r.length_squared() * 60.) + tangent * self.spin * 3.;
                q.velocity += (force * dt)
                    + if self.grabbed && moved {
                        delta * influence * 3.
                    } else {
                        Vec2::ZERO
                    };
                q.velocity *= math::exp(-2.2 * dt);
                q.velocity = q.velocity.clamp_length_max(2.);
                q.position += q.velocity * dt * if reduced { 0.35 } else { 1. };
                q.position = q.position.clamp(
                    -self.bounds + Vec2::splat(0.03),
                    self.bounds - Vec2::splat(0.03),
                );
            } else if self.kind == 4 {
                if self.ticks == 1 {
                    q.position = Vec2::new(
                        (unit(i, 1) * 2. - 1.) * self.bounds.x,
                        (unit(i, 2) * 2. - 1.) * 0.92,
                    );
                }
                let r = q.position - self.object;
                if self.grabbed && moved {
                    let weight = math::exp(-r.length_squared() / 0.16);
                    q.velocity += delta * weight * 9.;
                }
                q.velocity *= math::exp(-0.9 * dt);
                q.velocity = q.velocity.clamp_length_max(2.5);
                q.position += q.velocity * dt * if reduced { 0.35 } else { 1. };
                if q.position.x.abs() > self.bounds.x {
                    q.position.x = -q.position.x.signum() * self.bounds.x;
                }
                if q.position.y.abs() > 1. {
                    q.position.y = -q.position.y.signum();
                }
            }
        }
        // Short-range separation keeps the water granular and gives it a body.
        // Radial attraction supplies the requested fixed center of gravity.
        if self.kind == 2 && self.energy > 0.005 {
            for i in 0..144 {
                for j in 0..i {
                    let d = self.particles[i].position - self.particles[j].position;
                    let r = d.length();
                    if r > 0.00001 && r < 0.024 {
                        let push = d / r * (0.024 - r) * 0.25;
                        self.particles[i].position += push;
                        self.particles[j].position -= push;
                    }
                }
            }
        }
    }
    pub fn pixels(&self) -> Vec<Pixel> {
        let mut out = vec![Pixel::default(); PIXELS];
        let earth_height =
            self.energy
                .max(if self.kind == 3 && !self.earth_legacy && self.grabbed {
                    self.charge * 0.65
                } else {
                    0.
                });
        let time = if self.reduced {
            0.
        } else {
            self.ticks as f32 / 120.
        };
        for (i, v) in out.iter_mut().enumerate() {
            match self.kind {
                1 if self.energy > 0.005 => {
                    let age = (time * (0.55 + unit(i, 3) * 0.8) + unit(i, 4)).fract();
                    let width = SIZE * (1. - age * 0.75);
                    v.position = self.object
                        + Vec2::new(
                            (unit(i, 1) - 0.5) * width + math::sin(age * 8. + i as f32) * 0.025,
                            -SIZE * 0.2 - age * 0.55 * self.energy,
                        );
                    v.size = 0.014 + (1. - age) * 0.022;
                    v.color = if age < 0.25 {
                        [1., 0.7, 0.23]
                    } else if age < 0.6 {
                        [0.95, 0.24, 0.045]
                    } else {
                        [0.35, 0.045, 0.015]
                    };
                    v.visible = unit(i, 6) < self.energy && i < 160;
                }
                2 if i < 144 && self.energy > 0.005 => {
                    v.position = self.particles[i].position;
                    v.size = 0.026;
                    v.color = if i % 9 == 0 {
                        [0.52, 0.82, 0.95]
                    } else {
                        [0.035, 0.28 + unit(i, 4) * 0.18, 0.5 + unit(i, 2) * 0.18]
                    };
                    v.visible = true;
                }
                3 if earth_height > 0.005 && i < 120 => {
                    let x = (i % 12) as f32 - 5.5;
                    let row = (i / 12) as f32;
                    v.position = self.object + Vec2::new(x * 0.027, -row * 0.034 * earth_height);
                    v.size = 0.025;
                    let grey = 0.17 + unit(i, 2) * 0.2;
                    v.color = [grey, grey * 0.91, grey * 0.78];
                    v.visible = true;
                }
                4 => {
                    v.position = self.particles[i].position;
                    v.size = 0.007 + self.particles[i].velocity.length() * 0.006;
                    let glow = (0.12 + self.particles[i].velocity.length() * 0.4).min(0.85);
                    v.color = [glow * 0.88, glow * 0.95, glow];
                    v.visible = true;
                }
                _ => {}
            }
            // Quantized silhouettes, not smooth translucent point sprites.
            v.position = (v.position / 0.012).round() * 0.012;
        }
        // A quiet target mark teaches the earth finish without an opaque tutorial.
        if self.kind == 3 && self.earth_legacy && (self.phase == 2 || self.phase == 3) {
            for (j, v) in out[236..].iter_mut().enumerate() {
                *v = Pixel {
                    position: self.origin - Vec2::Y * 0.32
                        + Vec2::new(
                            if j % 2 == 0 { -0.065 } else { 0.065 },
                            if j < 2 { -0.065 } else { 0.065 },
                        ),
                    size: 0.018,
                    color: [0.65; 3],
                    visible: true,
                };
            }
        }
        out
    }
}
