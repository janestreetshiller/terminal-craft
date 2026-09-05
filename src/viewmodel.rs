//! Native first-person arm, held blocks and tools. No browser or image assets.
use crate::block::Block;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Hand,
    Block,
    Pickaxe,
    Axe,
    Shovel,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hand => "HAND",
            Self::Block => "BLOCK",
            Self::Pickaxe => "PICKAXE",
            Self::Axe => "AXE",
            Self::Shovel => "SHOVEL",
        }
    }

    pub fn mine_time(self, block: Block) -> f32 {
        let (base, preferred) = match block {
            Block::Slate
            | Block::Rust
            | Block::TsSlate
            | Block::Ruby
            | Block::Core
            | Block::Conduit
            | Block::Beacon => (1.2, Self::Pickaxe),
            Block::Wood | Block::Leaf => (0.8, Self::Axe),
            _ => (0.55, Self::Shovel),
        };
        if self == preferred {
            base * 0.30
        } else {
            base
        }
    }
}

#[derive(Default)]
pub struct ViewModel {
    pub tool: Tool,
    swing_left: f32,
    walk_phase: f32,
    walk_weight: f32,
    equip_left: f32,
    last_tool: Tool,
}

impl ViewModel {
    pub fn swing(&mut self) {
        if self.swing_left <= 0.0 {
            self.swing_left = 0.34;
        }
    }

    pub fn advance(&mut self, dt: f32, moving: bool) {
        self.swing_left = (self.swing_left - dt).max(0.0);
        let target = if moving { 1.0 } else { 0.0 };
        self.walk_weight += (target - self.walk_weight) * (1.0 - (-12.0 * dt).exp());
        if self.walk_weight < 0.001 {
            self.walk_weight = 0.0;
        }
        self.equip_left = (self.equip_left - dt).max(0.0);
        if self.tool != self.last_tool {
            self.last_tool = self.tool;
            self.equip_left = 0.20;
        }
        if moving {
            self.walk_phase = (self.walk_phase + dt * 9.0) % (2.0 * PI);
        }
    }

    pub fn draw(&self, rgba: &mut [u8], pw: i32, ph: i32, block: Block) {
        if pw < 2 || ph < 2 {
            return;
        }
        let progress = 1.0 - self.swing_left / 0.34;
        let swing = if self.swing_left <= 0.0 {
            0.0
        } else if progress < 0.18 {
            -0.10 * (progress / 0.18 * PI).sin()
        } else {
            ((progress - 0.18) / 0.82 * PI).sin().powf(0.8)
        };
        let bob = self.walk_phase.sin() * 0.035 * self.walk_weight;
        let equip = (self.equip_left / 0.20 * PI * 0.5).sin() * 0.55;
        let scale = ph as f32 * 0.25;
        let angle = -0.13 - swing * 0.8;
        let cx = pw as f32 * 0.79 - swing * scale * 0.55;
        let cy = ph as f32 * 0.82 + (bob + equip + swing * 0.12) * scale;
        let mut poly = |points: &[(f32, f32)], color| {
            let points: Vec<(f32, f32)> = points
                .iter()
                .map(|&(x, y)| {
                    (
                        cx + (x * angle.cos() - y * angle.sin()) * scale,
                        cy + (x * angle.sin() + y * angle.cos()) * scale,
                    )
                })
                .collect();
            polygon(rgba, pw, ph, &points, color);
        };
        // Sleeve and forearm extend beyond the lower-right viewport edge.
        poly(
            &[(-0.10, 0.18), (0.35, -0.03), (1.15, 1.7), (0.45, 1.7)],
            (39, 86, 96),
        );
        poly(
            &[(0.24, 0.02), (0.35, -0.03), (1.15, 1.7), (0.89, 1.7)],
            (25, 59, 71),
        );
        poly(
            &[(-0.22, -0.36), (0.17, -0.49), (0.44, 0.08), (-0.06, 0.28)],
            (187, 131, 93),
        );
        poly(
            &[
                (-0.22, -0.36),
                (0.06, -0.48),
                (0.17, -0.49),
                (0.27, -0.28),
                (-0.12, -0.12),
            ],
            (226, 174, 125),
        );
        poly(
            &[(0.17, -0.49), (0.32, -0.35), (0.44, 0.08), (0.26, 0.17)],
            (142, 91, 68),
        );
        match self.tool {
            Tool::Hand => {
                // Fingers and thumb, rather than a block painted as a hand.
                poly(
                    &[
                        (-0.22, -0.35),
                        (-0.10, -0.40),
                        (0.03, -0.07),
                        (-0.08, -0.02),
                    ],
                    (204, 148, 105),
                );
                poly(
                    &[(-0.03, -0.44), (0.03, -0.46), (0.15, -0.14), (0.10, -0.12)],
                    (161, 103, 73),
                );
                poly(
                    &[(0.08, -0.47), (0.13, -0.49), (0.25, -0.19), (0.20, -0.17)],
                    (161, 103, 73),
                );
            }
            Tool::Block => {
                poly(
                    &[
                        (-0.65, -0.82),
                        (-0.20, -1.02),
                        (0.26, -0.79),
                        (-0.19, -0.57),
                    ],
                    block.rgb(0),
                );
                poly(
                    &[
                        (-0.65, -0.82),
                        (-0.19, -0.57),
                        (-0.19, -0.08),
                        (-0.65, -0.33),
                    ],
                    block.rgb(2),
                );
                poly(
                    &[(-0.19, -0.57), (0.26, -0.79), (0.26, -0.30), (-0.19, -0.08)],
                    block.rgb(4),
                );
            }
            Tool::Pickaxe | Tool::Axe | Tool::Shovel => {
                // Oak handle gripped by the near hand.
                poly(
                    &[(-0.42, -1.05), (-0.29, -1.11), (0.26, 0.22), (0.12, 0.29)],
                    (113, 73, 39),
                );
                poly(
                    &[(-0.42, -1.05), (-0.38, -1.07), (0.17, 0.27), (0.12, 0.29)],
                    (190, 137, 70),
                );
                match self.tool {
                    Tool::Pickaxe => {
                        poly(
                            &[
                                (-1.03, -0.92),
                                (-0.88, -1.22),
                                (-0.42, -1.40),
                                (0.14, -1.19),
                                (0.33, -0.91),
                                (-0.28, -1.16),
                                (-0.79, -1.02),
                            ],
                            (65, 78, 85),
                        );
                        poly(
                            &[
                                (-0.96, -0.98),
                                (-0.85, -1.17),
                                (-0.43, -1.33),
                                (0.08, -1.15),
                                (0.20, -1.00),
                                (-0.30, -1.23),
                                (-0.81, -1.08),
                            ],
                            (182, 207, 205),
                        );
                    }
                    Tool::Axe => {
                        poly(
                            &[
                                (-0.37, -1.42),
                                (-0.88, -1.40),
                                (-1.02, -1.02),
                                (-0.80, -0.72),
                                (-0.34, -0.98),
                            ],
                            (74, 89, 95),
                        );
                        poly(
                            &[
                                (-0.45, -1.36),
                                (-0.85, -1.35),
                                (-0.95, -1.04),
                                (-0.78, -0.82),
                                (-0.41, -1.02),
                            ],
                            (188, 213, 208),
                        );
                    }
                    Tool::Shovel => {
                        poly(
                            &[
                                (-0.70, -1.61),
                                (-0.24, -1.70),
                                (-0.07, -1.22),
                                (-0.28, -0.92),
                                (-0.60, -1.14),
                            ],
                            (79, 94, 100),
                        );
                        poly(
                            &[
                                (-0.63, -1.54),
                                (-0.29, -1.60),
                                (-0.17, -1.23),
                                (-0.30, -1.03),
                                (-0.53, -1.19),
                            ],
                            (190, 213, 203),
                        );
                    }
                    _ => unreachable!(),
                }
                poly(
                    &[(-0.14, -0.17), (0.05, -0.25), (0.18, 0.01), (-0.03, 0.10)],
                    (224, 170, 120),
                );
            }
        }
    }
}

fn polygon(rgba: &mut [u8], width: i32, height: i32, vertices: &[(f32, f32)], color: (u8, u8, u8)) {
    let min_y = vertices
        .iter()
        .map(|p| p.1.floor() as i32)
        .min()
        .unwrap_or(0)
        .max(0);
    let max_y = vertices
        .iter()
        .map(|p| p.1.ceil() as i32)
        .max()
        .unwrap_or(0)
        .min(height - 1);
    for y in min_y..=max_y {
        let scan = y as f32 + 0.5;
        let mut xs = [0.0_f32; 16];
        let mut count = 0;
        debug_assert!(vertices.len() <= xs.len());
        for i in 0..vertices.len() {
            let (ax, ay) = vertices[i];
            let (bx, by) = vertices[(i + 1) % vertices.len()];
            if (ay <= scan && by > scan) || (by <= scan && ay > scan) {
                xs[count] = ax + (scan - ay) / (by - ay) * (bx - ax);
                count += 1;
            }
        }
        let xs = &mut xs[..count];
        xs.sort_unstable_by(|a, b| a.total_cmp(b));
        for pair in xs.as_chunks::<2>().0 {
            for x in (pair[0].ceil() as i32).max(0)..(pair[1].ceil() as i32).min(width) {
                let i = ((y * width + x) * 4) as usize;
                if i + 3 < rgba.len() {
                    rgba[i..i + 4].copy_from_slice(&[color.0, color.1, color.2, 255]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;

    fn image(view: &ViewModel) -> Vec<u8> {
        let mut rgba = vec![0; 320 * 200 * 4];
        view.draw(&mut rgba, 320, 200, Block::Terra);
        rgba
    }

    #[test]
    fn hand_and_every_tool_have_distinct_visible_geometry() {
        let mut images = Vec::new();
        for tool in [
            Tool::Hand,
            Tool::Block,
            Tool::Pickaxe,
            Tool::Axe,
            Tool::Shovel,
        ] {
            let view = ViewModel {
                tool,
                ..ViewModel::default()
            };
            let rgba = image(&view);
            assert!(rgba.as_chunks::<4>().0.iter().filter(|p| p[3] != 0).count() > 100);
            assert!(
                !images.contains(&rgba),
                "tool {tool:?} duplicated another model"
            );
            images.push(rgba);
        }
    }

    #[test]
    fn motion_settles_and_equipping_has_weight_instead_of_snapping() {
        let mut view = ViewModel::default();
        let idle = image(&view);
        view.advance(0.1, true);
        view.advance(0.01, false);
        assert!(
            image(&view) != idle,
            "stopping should settle rather than snap"
        );
        view.advance(2.0, false);
        assert!(image(&view) == idle);
        view.tool = Tool::Pickaxe;
        let static_tool = image(&view);
        view.advance(0.04, false);
        assert!(
            image(&view) != static_tool,
            "equipping should dip and return"
        );
        view.advance(1.0, false);
        assert!(image(&view) == static_tool);
    }

    #[test]
    fn hitting_animates_then_returns_to_idle() {
        let mut view = ViewModel::default();
        let idle = image(&view);
        view.swing();
        view.advance(0.10, false);
        assert_ne!(image(&view), idle);
        view.advance(1.0, false);
        assert_eq!(image(&view), idle);
    }

    #[test]
    fn correct_tools_mine_their_material_faster() {
        assert!(Tool::Pickaxe.mine_time(Block::Slate) < Tool::Hand.mine_time(Block::Slate));
        assert!(Tool::Axe.mine_time(Block::Wood) < Tool::Pickaxe.mine_time(Block::Wood));
        assert!(Tool::Shovel.mine_time(Block::Sand) < Tool::Axe.mine_time(Block::Sand));
    }

    #[test]
    fn holding_a_different_block_changes_the_render() {
        let view = ViewModel {
            tool: Tool::Block,
            ..ViewModel::default()
        };
        let a = image(&view);
        let mut b = vec![0; a.len()];
        view.draw(&mut b, 320, 200, Block::Core);
        assert_ne!(a, b);
    }
}
