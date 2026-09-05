// Numeric camera/rectangle APIs intentionally use explicit coordinate arguments.
#![allow(clippy::too_many_arguments)]
use crate::block::Block;
use crate::world::{World, SX, SY, SZ};

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub vy: f32,
    pub flying: bool,
    pub sprinting: bool,
    pub crouching: bool,
    pub on_ground: bool,
}

impl Player {
    pub fn spawn(world: &World) -> Self {
        let (x, y, z) = world.spawn();
        Self {
            x,
            y,
            z,
            yaw: 0.35,
            pitch: 0.12,
            vy: 0.0,
            flying: false,
            sprinting: false,
            crouching: false,
            on_ground: true,
        }
    }

    pub fn look_dir(&self) -> (f32, f32, f32) {
        let cp = self.pitch.cos();
        (self.yaw.cos() * cp, self.pitch.sin(), self.yaw.sin() * cp)
    }

    pub fn eye(&self) -> (f32, f32, f32) {
        (
            self.x,
            self.y + if self.crouching { 1.30 } else { 1.62 },
            self.z,
        )
    }

    pub fn look(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        let limit = std::f32::consts::FRAC_PI_2 - 0.001;
        self.pitch = (self.pitch + dpitch).clamp(-limit, limit);
    }

    pub fn tick(
        &mut self,
        world: &World,
        mx: f32,
        mz: f32,
        jump: bool,
        sneak: bool,
        space: bool,
        dt: f32,
        creative: bool,
    ) {
        let old_y = self.y;
        if creative && self.flying {
            self.vy = 0.0;
            if space {
                self.y += 8.0 * dt;
            }
            if sneak {
                self.y -= 8.0 * dt;
            }
        } else {
            if jump && self.on_ground {
                self.vy = 9.2;
                self.on_ground = false;
            }
            self.vy -= 28.0 * dt;
            if self.vy < -25.0 {
                self.vy = -25.0;
            }
            self.y += self.vy * dt;
        }

        self.crouching = !self.flying && sneak;
        let speed = if self.flying {
            if self.sprinting {
                12.0
            } else {
                8.0
            }
        } else if self.crouching {
            1.6
        } else if self.sprinting {
            4.3 * 1.35
        } else {
            4.3
        };
        let (fx, _, fz) = {
            let (lx, _, lz) = (self.yaw.cos(), 0.0, self.yaw.sin());
            let rx = -lz;
            let rz = lx;
            (lx * mz + rx * mx, 0.0, lz * mz + rz * mx)
        };
        let len = (fx * fx + fz * fz).sqrt();
        let (fx, fz) = if len > 0.001 {
            (fx / len * speed * dt, fz / len * speed * dt)
        } else {
            (0.0, 0.0)
        };

        // Y collision after gravity
        if self.collides(world, self.x, self.y, self.z) {
            if self.y < old_y {
                self.y = self.y.floor() + 0.001;
                // climb out of floor
                while self.collides(world, self.x, self.y, self.z) {
                    self.y += 0.05;
                    if self.y > SY as f32 {
                        break;
                    }
                }
                self.vy = 0.0;
                self.on_ground = true;
            } else {
                self.y = old_y;
                self.vy = 0.0;
            }
        } else {
            // standing on block?
            self.on_ground = self.collides(world, self.x, self.y - 0.08, self.z);
        }

        // Resolve gravity first; otherwise every horizontal step collides with
        // the temporary floor penetration and grounded movement stalls.
        self.try_move(world, fx, 0.0, 0.0);
        self.try_move(world, 0.0, 0.0, fz);
        self.x = self.x.clamp(1.2, SX as f32 - 1.2);
        self.z = self.z.clamp(1.2, SZ as f32 - 1.2);
        self.y = self.y.clamp(1.0, SY as f32 - 3.0);
    }

    fn try_move(&mut self, world: &World, dx: f32, dy: f32, dz: f32) {
        let nx = self.x + dx;
        let ny = self.y + dy;
        let nz = self.z + dz;
        if !self.collides(world, nx, ny, nz) {
            self.x = nx;
            self.y = ny;
            self.z = nz;
        }
    }

    fn collides(&self, world: &World, x: f32, y: f32, z: f32) -> bool {
        let hw = 0.3;
        let h = 1.75;
        let xs = [(x - hw).floor() as i32, (x + hw).floor() as i32];
        let zs = [(z - hw).floor() as i32, (z + hw).floor() as i32];
        let ys = [
            y.floor() as i32,
            (y + h * 0.5).floor() as i32,
            (y + h).floor() as i32,
        ];
        for yi in ys {
            for xi in xs {
                for zi in zs {
                    if world.get(xi, yi, zi).is_solid() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

pub struct Hit {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub face: u8,
    pub px: i32,
    pub py: i32,
    pub pz: i32,
    pub block: Block,
    pub dist: f32,
}

/// DDA raycast. Returns first solid hit within `reach`.
pub fn raycast(
    world: &World,
    ox: f32,
    oy: f32,
    oz: f32,
    dx: f32,
    dy: f32,
    dz: f32,
    reach: f32,
) -> Option<Hit> {
    let mut ix = ox.floor() as i32;
    let mut iy = oy.floor() as i32;
    let mut iz = oz.floor() as i32;

    let step_x = if dx > 0.0 { 1 } else { -1 };
    let step_y = if dy > 0.0 { 1 } else { -1 };
    let step_z = if dz > 0.0 { 1 } else { -1 };

    let tdx = if dx.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / dx).abs()
    };
    let tdy = if dy.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / dy).abs()
    };
    let tdz = if dz.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / dz).abs()
    };

    let mut tmax_x = if dx.abs() < 1e-8 {
        f32::INFINITY
    } else if dx > 0.0 {
        (ix as f32 + 1.0 - ox) / dx
    } else {
        (ix as f32 - ox) / dx
    };
    let mut tmax_y = if dy.abs() < 1e-8 {
        f32::INFINITY
    } else if dy > 0.0 {
        (iy as f32 + 1.0 - oy) / dy
    } else {
        (iy as f32 - oy) / dy
    };
    let mut tmax_z = if dz.abs() < 1e-8 {
        f32::INFINITY
    } else if dz > 0.0 {
        (iz as f32 + 1.0 - oz) / dz
    } else {
        (iz as f32 - oz) / dz
    };

    let mut face = 0u8;
    let mut t = 0.0f32;
    let mut px = ix;
    let mut py = iy;
    let mut pz = iz;

    for _ in 0..96 {
        if t > reach {
            return None;
        }
        let b = world.get(ix, iy, iz);
        if b.is_solid() {
            return Some(Hit {
                x: ix,
                y: iy,
                z: iz,
                face,
                px,
                py,
                pz,
                block: b,
                dist: t,
            });
        }
        px = ix;
        py = iy;
        pz = iz;
        if tmax_x < tmax_y && tmax_x < tmax_z {
            ix += step_x;
            t = tmax_x;
            tmax_x += tdx;
            face = if step_x > 0 { 3 } else { 2 };
        } else if tmax_y < tmax_z {
            iy += step_y;
            t = tmax_y;
            tmax_y += tdy;
            face = if step_y > 0 { 1 } else { 0 };
        } else {
            iz += step_z;
            t = tmax_z;
            tmax_z += tdz;
            face = if step_z > 0 { 5 } else { 4 };
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_vertical_look_and_flight_respect_solid_ceilings() {
        let mut world = World::generate(42);
        for x in 15..25 {
            for z in 15..25 {
                for y in 15..25 {
                    world.set(x, y, z, Block::Air);
                }
                world.set(x, 15, z, Block::Slate);
                world.set(x, 19, z, Block::Slate);
            }
        }
        let mut p = Player::spawn(&world);
        p.x = 20.5;
        p.z = 20.5;
        p.y = 16.001;
        p.flying = true;
        p.look(0.0, 100.0);
        assert!(p.pitch > 1.55);
        p.look(0.0, -200.0);
        assert!(p.pitch < -1.55);
        for _ in 0..40 {
            p.tick(&world, 0.0, 0.0, false, false, true, 0.016, true);
            assert!(!p.collides(&world, p.x, p.y, p.z));
        }
        assert!(p.y < 18.0);
        for _ in 0..80 {
            p.tick(&world, 0.0, 0.0, false, true, false, 0.016, true);
        }
        assert!(p.y >= 16.0 && p.y < 16.1);
    }

    #[test]
    fn sprint_sneak_and_diagonal_motion_have_consistent_speeds() {
        let mut world = World::generate(42);
        for x in 15..30 {
            for z in 15..30 {
                for y in 15..25 {
                    world.set(x, y, z, if y == 15 { Block::Slate } else { Block::Air });
                }
            }
        }
        let run = |sprint: bool, sneak: bool, diagonal: bool| {
            let mut p = Player::spawn(&world);
            p.x = 20.5;
            p.y = 16.001;
            p.z = 20.5;
            p.yaw = 0.0;
            p.sprinting = sprint;
            for _ in 0..10 {
                p.tick(
                    &world,
                    if diagonal { 1.0 } else { 0.0 },
                    1.0,
                    false,
                    sneak,
                    false,
                    1.0 / 60.0,
                    false,
                );
            }
            (
                ((p.x - 20.5).powi(2) + (p.z - 20.5).powi(2)).sqrt(),
                p.eye().1 - p.y,
            )
        };
        let (walk, eye) = run(false, false, false);
        let (sprint, _) = run(true, false, false);
        let (sneak, low_eye) = run(false, true, false);
        let (diagonal, _) = run(false, false, true);
        assert!(sprint > walk * 1.3);
        assert!(sneak < walk * 0.5);
        assert!(low_eye < eye);
        assert!((walk - diagonal).abs() < 0.001);
    }

    #[test]
    fn walking_on_flat_ground_does_not_require_jumping() {
        let mut world = World::generate(42);
        for x in 15..30 {
            for z in 15..30 {
                for y in 14..25 {
                    world.set(x, y, z, if y == 19 { Block::Slate } else { Block::Air });
                }
            }
        }
        let mut p = Player::spawn(&world);
        p.x = 20.5;
        p.y = 20.001;
        p.z = 20.5;
        p.yaw = 0.0;
        p.on_ground = true;
        for _ in 0..10 {
            p.tick(&world, 0.0, 1.0, false, false, false, 0.033, false);
        }
        assert!(p.x > 21.0, "walking stalled at {}", p.x);
        assert!((p.y - 20.0).abs() < 0.08);
    }
}
