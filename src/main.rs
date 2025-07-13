use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};

use piston::WindowSettings;
use piston_window::{ellipse::circle, *};

const BASE_PARTICLE_RADIUS: f64 = 10.0;
const CELL_SIZE: f64 = BASE_PARTICLE_RADIUS * 2.0;
const PARTICLE_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const GRAVITY: f64 = 100.0;

trait Distance<T> {
    fn distance(self, other: Self) -> T;
}

impl Distance<f64> for [f64; 2] {
    fn distance(self, other: Self) -> f64 {
        f64::sqrt((self[0] - other[0]).powi(2) + (self[1] - other[1]).powi(2))
    }
}

trait Direction<T> {
    fn direction(self, other: Self) -> [T; 2];
}

impl Direction<f64> for [f64; 2] {
    fn direction(self, other: Self) -> [f64; 2] {
        let distance = self.distance(other);
        if f64::is_subnormal(distance * 65536.0) {
            return [0.0, 0.0];
        }

        [
            (self[0] - other[0]) / distance,
            (self[1] - other[1]) / distance,
        ]
    }
}

#[derive(Clone, Copy)]
struct Particle {
    pos: [f64; 2],
    velocity: [f64; 2],
}

impl Particle {
    fn push_out_of_border(&mut self, size: [f64; 2]) {
        self.pos[0] = self.pos[0].clamp(0.0, size[0]);
        self.pos[1] = self.pos[1].clamp(0.0, size[0]);
    }

    fn simulate(&mut self, dt: f64, size: [f64; 2]) {
        self.pos[0] += self.velocity[0] * dt;
        self.pos[1] += self.velocity[1] * dt;

        self.velocity[1] += GRAVITY * dt;

        self.push_out_of_border(size);
    }
}

#[derive(Default, Debug)]
struct Grid<T>(HashMap<[u32; 2], T>);

#[derive(Default, Debug)]
struct ParticleGrid(Grid<HashSet<usize>>);

trait GridParticleInterface {
    const OFFSET: [f64; 2];

    fn get_grid_pos(particle_pos: [f64; 2]) -> [u32; 2] {
        return [
            (particle_pos[0] / CELL_SIZE + Self::OFFSET[0]).floor() as u32,
            (particle_pos[1] / CELL_SIZE + Self::OFFSET[1]).floor() as u32,
        ];
    }
    fn remove_grid_pos(&mut self, grid_pos: [u32; 2]);
    fn remove(&mut self, pos: [f64; 2]) {
        self.remove_grid_pos(Self::get_grid_pos(pos))
    }
    fn offset(pos: [f64; 2]) -> [f64; 2] {
        let pos = [
            pos[0] / CELL_SIZE + Self::OFFSET[0],
            pos[1] / CELL_SIZE + Self::OFFSET[1],
        ];

        let grid_pos = pos.map(f64::floor);
        [pos[0] - grid_pos[0], pos[1] - grid_pos[1]]
    }
    fn get_weights(pos: [f64; 2]) -> [f64; 4] {
        let offset = Self::offset(pos);

        [
            offset[0] * offset[1],
            (1.0 - offset[0]) * offset[1],
            offset[0] * (1.0 - offset[1]),
            (1.0 - offset[0]) * (1.0 - offset[1]),
        ]
    }
}

impl ParticleGrid {
    fn move_particle_grid_pos(
        &mut self,
        old: [u32; 2],
        new: [u32; 2],
        particle_idx: usize,
    ) -> bool {
        let mut remove_old = false;

        if let Some(old_cell) = self.0 .0.get_mut(&old) {
            old_cell.remove(&particle_idx);
            remove_old = old_cell.is_empty();
        }

        if remove_old {
            self.0 .0.remove(&old);
        }

        if let Some(new_cell) = self.0 .0.get_mut(&new) {
            new_cell.insert(particle_idx);
        } else {
            let mut new_set = HashSet::new();
            new_set.insert(particle_idx);
            self.0 .0.insert(new, new_set);
        }

        remove_old
    }

    fn move_particle(&mut self, old: [f64; 2], new: [f64; 2], particle_idx: usize) -> bool {
        self.move_particle_grid_pos(
            Self::get_grid_pos(old),
            Self::get_grid_pos(new),
            particle_idx,
        )
    }
}

impl GridParticleInterface for ParticleGrid {
    const OFFSET: [f64; 2] = [0.0, 0.0];

    fn remove_grid_pos(&mut self, grid_pos: [u32; 2]) {
        self.0 .0.remove(&grid_pos);
    }
}

#[derive(Default)]
struct XVelocityGrid(Grid<f64>);

impl XVelocityGrid {
    fn particle_to_cell(&mut self, velocity: f64, pos: [f64; 2]) {
        let grid_pos = Self::get_grid_pos(pos);
        let neighbours = [
            [grid_pos[0], grid_pos[1]],
            [grid_pos[0] + 1, grid_pos[1]],
            [grid_pos[0], grid_pos[1] + 1],
            [grid_pos[0] + 1, grid_pos[1] + 1],
        ];
        let weights = Self::get_weights(pos);
        for (neighbour, weight) in neighbours.iter().zip(weights.iter()) {
            *self.0 .0.entry(*neighbour).or_insert(0.0) += *weight * velocity;
        }
    }
}

impl GridParticleInterface for XVelocityGrid {
    const OFFSET: [f64; 2] = [0.0, 0.5];

    fn remove_grid_pos(&mut self, grid_pos: [u32; 2]) {
        self.0 .0.remove(&grid_pos);
    }
}

#[derive(Default)]
struct YVelocityGrid(Grid<f64>);

impl YVelocityGrid {
    fn particle_to_cell(&mut self, velocity: f64, pos: [f64; 2]) {
        let grid_pos = Self::get_grid_pos(pos);
        let neighbours = [
            [grid_pos[0], grid_pos[1]],
            [grid_pos[0] + 1, grid_pos[1]],
            [grid_pos[0], grid_pos[1] + 1],
            [grid_pos[0] + 1, grid_pos[1] + 1],
        ];
        let weights = Self::get_weights(pos);
        for (neighbour, weight) in neighbours.iter().zip(weights.iter()) {
            *self.0 .0.entry(*neighbour).or_insert(0.0) += *weight * velocity;
        }
    }
}

impl GridParticleInterface for YVelocityGrid {
    const OFFSET: [f64; 2] = [0.5, 0.0];

    fn remove_grid_pos(&mut self, grid_pos: [u32; 2]) {
        self.0 .0.remove(&grid_pos);
    }
}
struct Simulation {
    particles: Vec<Particle>,
    x_velocity: XVelocityGrid,
    y_velocity: YVelocityGrid,
    particle_grid: ParticleGrid,
    size: [u32; 2],
}

impl Simulation {
    fn new(size: [u32; 2]) -> Simulation {
        Simulation {
            particles: vec![],
            x_velocity: XVelocityGrid::default(),
            y_velocity: YVelocityGrid::default(),
            particle_grid: ParticleGrid::default(),
            size,
        }
    }

    fn spawn(&mut self, particle: Particle) {
        self.particles.push(particle);
        let new_index = self.particles.len() - 1;
        let grid_pos = ParticleGrid::get_grid_pos(particle.pos);
        if let Some(hashset) = self.particle_grid.0 .0.get_mut(&grid_pos) {
            hashset.insert(new_index);
        } else {
            let mut new_hashset = HashSet::new();
            new_hashset.insert(new_index);
            self.particle_grid.0 .0.insert(grid_pos, new_hashset);
        }
    }

    fn self_collide_particles(&mut self) {
        let mut collisions = HashMap::new();

        for (pos, particles) in &self.particle_grid.0 .0 {
            let other_particles = (-1i32..1i32)
                .flat_map(|x| (-1i32..1i32).map(move |y| [x, y]))
                .map(|delta_pos| {
                    [
                        (pos[0] as i32 + delta_pos[0]) as u32,
                        (pos[1] as i32 + delta_pos[1]) as u32,
                    ]
                })
                .filter_map(|neighbour_pos| self.particle_grid.0 .0.get(&neighbour_pos))
                .flat_map(|neighbour| neighbour.iter())
                .chain(particles.iter());

            for particle_idx in particles {
                for other_particle_idx in other_particles.clone() {
                    if particle_idx == other_particle_idx {
                        continue;
                    };

                    let key = if particle_idx < other_particle_idx {
                        [*other_particle_idx, *particle_idx]
                    } else {
                        [*particle_idx, *other_particle_idx]
                    };

                    if collisions.contains_key(&key) {
                        continue;
                    }

                    let particle = self.particles[*particle_idx];
                    let other_particle = self.particles[*other_particle_idx];

                    let distance = particle.pos.distance(other_particle.pos);
                    let direction = if particle_idx < other_particle_idx {
                        other_particle.pos.direction(particle.pos)
                    } else {
                        particle.pos.direction(other_particle.pos)
                    };

                    if distance < 2.0 * BASE_PARTICLE_RADIUS {
                        collisions.insert(key, (distance, direction));
                    }
                }
            }
        }

        for (idxs, (distance, direction)) in collisions {
            let shift = BASE_PARTICLE_RADIUS - distance / 2.0;
            let old_grid_pos = idxs.map(|idx| ParticleGrid::get_grid_pos(self.particles[idx].pos));

            self.particles[idxs[0]].pos[0] += shift * direction[0];
            self.particles[idxs[1]].pos[0] -= shift * direction[0];
            self.particles[idxs[0]].pos[1] += shift * direction[1];
            self.particles[idxs[1]].pos[1] -= shift * direction[1];

            let new_grid_pos = idxs.map(|idx| ParticleGrid::get_grid_pos(self.particles[idx].pos));

            idxs.iter()
                .zip(old_grid_pos.iter())
                .zip(new_grid_pos.iter())
                .filter(|((_, old), new)| old != new)
                .map(|((idx, old), new)| {
                    self.particle_grid.move_particle_grid_pos(*old, *new, *idx)
                })
                .for_each(drop);

            self.particles[idxs[0]].velocity[0] += shift * direction[0] * 4.0;
            self.particles[idxs[1]].velocity[0] -= shift * direction[0] * 4.0;
            self.particles[idxs[0]].velocity[1] += shift * direction[1] * 4.0;
            self.particles[idxs[1]].velocity[1] -= shift * direction[1] * 4.0;
        }
    }

    fn simulate_particles(&mut self, dt: f64) {
        for (idx, particle) in self.particles.iter_mut().enumerate() {
            let old_grid_pos = ParticleGrid::get_grid_pos(particle.pos);

            let size = [
                self.size[0] as f64 * CELL_SIZE,
                self.size[1] as f64 * CELL_SIZE,
            ];
            particle.simulate(dt, size);

            let new_grid_pos = ParticleGrid::get_grid_pos(particle.pos);

            if old_grid_pos == new_grid_pos {
                continue;
            }

            self.particle_grid
                .move_particle_grid_pos(old_grid_pos, new_grid_pos, idx);
        }

        self.self_collide_particles();
    }

    fn particle_to_grid_velocity(&mut self) {
        self.x_velocity.0 .0.clear();
        self.y_velocity.0 .0.clear();

        for particle in &self.particles {
            self.x_velocity
                .particle_to_cell(particle.velocity[0], particle.pos);
            self.y_velocity
                .particle_to_cell(particle.velocity[1], particle.pos);
        }
    }

    fn make_incompressible(&mut self) {
        // TODO
    }

    fn grid_to_particle_velocity(&mut self) {
        // TODO
    }

    fn simulate(&mut self, dt: f64) {
        // self.simulate_particles(dt);
        self.particle_to_grid_velocity();
        self.make_incompressible();
        self.grid_to_particle_velocity();
    }

    fn render<G: Graphics>(&self, ctx: Context, graphics_buffer: &mut G) {
        for particle in &self.particles {
            ellipse(
                PARTICLE_COLOR,
                circle(particle.pos[0], particle.pos[1], BASE_PARTICLE_RADIUS),
                ctx.transform,
                graphics_buffer,
            );
        }
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Fluid simulation", [1000, 1000])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut simulation = Simulation::new([10, 10]);
    for x in 0..10 {
        for y in 0..10 {
            simulation.spawn(Particle {
                pos: [
                    100.0 + x as f64 * BASE_PARTICLE_RADIUS,
                    100.0 + y as f64 * BASE_PARTICLE_RADIUS,
                ],
                velocity: [0.0, 0.0],
            });
        }
    }

    window.set_lazy(false);
    let mut prev_frame = SystemTime::now();
    while let Some(event) = window.next() {
        window.draw_2d(&event, |ctx, graphics_buffer, _device| {
            let dt = SystemTime::now()
                .duration_since(prev_frame)
                .expect("Time may have gone backwatds");
            simulation.simulate(dt.as_secs_f64());
            graphics_buffer.clear_color([1.0, 1.0, 1.0, 1.0]);
            simulation.render(ctx, graphics_buffer);
            prev_frame = SystemTime::now();
        });
    }
}
