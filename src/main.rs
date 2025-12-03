use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    time::SystemTime,
};

use piston::WindowSettings;
use piston_window::{ellipse::circle, *};

const BASE_PARTICLE_RADIUS: f64 = 10.0;
const CELL_SIZE: f64 = BASE_PARTICLE_RADIUS * 2.0;
const PARTICLE_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const GRAVITY: f64 = 10.0;
const NUM_PARTICLE_ITERS: usize = 10;
const COLLISION_RANDOMNESS: f64 = 0.1;

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

#[derive(Clone, Copy, Debug)]
struct Particle {
    pos: [f64; 2],
    velocity: [f64; 2],
}

impl Particle {
    fn push_out_of_border(&mut self, size: [f64; 2]) {
        if self.pos[0] > size[0] || self.pos[0] <= 0.0 {
            self.velocity[0] = 0.0
        }

        if self.pos[1] > size[1] || self.pos[1] <= 0.0 {
            self.velocity[1] = 0.0
        }

        self.pos[0] = self.pos[0].clamp(0.0, size[0]);
        self.pos[1] = self.pos[1].clamp(0.0, size[1]);
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

    fn get_grid_pos_continuous(particle_pos: [f64; 2]) -> [f64; 2] {
        [
            particle_pos[0] / CELL_SIZE + Self::OFFSET[0],
            particle_pos[1] / CELL_SIZE + Self::OFFSET[1],
        ]
    }
    fn get_grid_pos(particle_pos: [f64; 2]) -> [u32; 2] {
        Self::get_grid_pos_continuous(particle_pos).map(|x| x.floor() as u32)
    }
    fn offset(pos: [f64; 2]) -> [f64; 2] {
        let pos = Self::get_grid_pos_continuous(pos);
        let grid_pos = pos.map(f64::floor);

        [pos[0] - grid_pos[0], pos[1] - grid_pos[1]]
    }
    fn get_weights(pos: [f64; 2]) -> [f64; 4] {
        let offset = Self::offset(pos);

        [
            (1.0 - offset[0]) * (1.0 - offset[1]),
            offset[0] * (1.0 - offset[1]),
            (1.0 - offset[0]) * offset[1],
            offset[0] * offset[1],
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

        if let Some(old_cell) = self.0.0.get_mut(&old) {
            old_cell.remove(&particle_idx);
            remove_old = old_cell.is_empty();
        }

        if remove_old {
            self.0.0.remove(&old);
        }

        if let Some(new_cell) = self.0.0.get_mut(&new) {
            new_cell.insert(particle_idx);
        } else {
            let mut new_set = HashSet::new();
            new_set.insert(particle_idx);
            self.0.0.insert(new, new_set);
        }

        remove_old
    }
}

impl GridParticleInterface for ParticleGrid {
    const OFFSET: [f64; 2] = [0.0, 0.0];
}

trait PosDirection {}

#[derive(Clone, Copy, Default, Debug)]
struct X;

impl PosDirection for X {}

#[derive(Clone, Copy, Default, Debug)]
struct Y;

impl PosDirection for Y {}

#[derive(Default, Debug)]
struct VelocityGrid<P: PosDirection>(Grid<f64>, PhantomData<P>);

impl<P: PosDirection> VelocityGrid<P>
where
    VelocityGrid<P>: GridParticleInterface,
{
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
            *self.0.0.entry(*neighbour).or_insert(0.0) += *weight * velocity;
        }
    }

    fn grid_to_particle(&self, particle_pos: [f64; 2]) -> f64 {
        let grid_pos = Self::get_grid_pos(particle_pos);
        let neighbours = [
            [grid_pos[0], grid_pos[1]],
            [grid_pos[0] + 1, grid_pos[1]],
            [grid_pos[0], grid_pos[1] + 1],
            [grid_pos[0] + 1, grid_pos[1] + 1],
        ];

        let weights = Self::get_weights(particle_pos);
        let mut total = 0.0;

        for (neighbour, weight) in neighbours.iter().zip(weights.iter()) {
            total += self.0.0.get(neighbour).cloned().unwrap_or(0.0) * *weight;
        }
        total
    }
}

impl GridParticleInterface for VelocityGrid<X> {
    const OFFSET: [f64; 2] = [0.0, 0.5];
}

impl GridParticleInterface for VelocityGrid<Y> {
    const OFFSET: [f64; 2] = [0.5, 0.0];
}

struct Simulation {
    particles: Vec<Particle>,
    x_velocity: VelocityGrid<X>,
    y_velocity: VelocityGrid<Y>,
    particle_grid: ParticleGrid,
    size: [u32; 2],
}

impl Simulation {
    fn new(size: [u32; 2]) -> Simulation {
        Simulation {
            particles: vec![],
            x_velocity: VelocityGrid::<X>::default(),
            y_velocity: VelocityGrid::<Y>::default(),
            particle_grid: ParticleGrid::default(),
            size,
        }
    }

    fn spawn(&mut self, particle: Particle) {
        self.particles.push(particle);
        let new_index = self.particles.len() - 1;
        let grid_pos = ParticleGrid::get_grid_pos(particle.pos);
        if let Some(hashset) = self.particle_grid.0.0.get_mut(&grid_pos) {
            hashset.insert(new_index);
        } else {
            let mut new_hashset = HashSet::new();
            new_hashset.insert(new_index);
            self.particle_grid.0.0.insert(grid_pos, new_hashset);
        }
    }

    fn get_particle_collision(&self) -> HashSet<[usize; 2]> {
        let mut collisions = HashSet::new();

        for (pos, particles) in &self.particle_grid.0.0 {
            let other_particles = (-1i32..=1i32)
                .flat_map(|x| (-1i32..=1i32).map(move |y| [x, y]))
                .map(|delta_pos| {
                    [
                        (pos[0] as i32 + delta_pos[0]) as u32,
                        (pos[1] as i32 + delta_pos[1]) as u32,
                    ]
                })
                .filter_map(|neighbour_pos| self.particle_grid.0.0.get(&neighbour_pos))
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

                    if collisions.contains(&key) {
                        continue;
                    }

                    let particle = self.particles[*particle_idx];
                    let other_particle = self.particles[*other_particle_idx];

                    let distance = particle.pos.distance(other_particle.pos);
                    if distance < 2.0 * BASE_PARTICLE_RADIUS {
                        collisions.insert(key);
                    }
                }
            }
        }

        collisions
    }

    fn self_collide_particles(&mut self) {
        let collisions = self.get_particle_collision();

        for idxs in collisions {
            let random_shift = [0.0, 0.0].direction([
                (idxs[0] as f64 * 10000.0 / 12345.0 % 1.0),
                (idxs[1] as f64 * 10000.0 / 12345.0 % 1.0),
            ]);
            let mut distance = self.particles[idxs[0]]
                .pos
                .distance(self.particles[idxs[1]].pos);
            let mut direction = self.particles[idxs[0]]
                .pos
                .direction(self.particles[idxs[1]].pos);
            if distance.is_subnormal() || distance == 0.0 {
                distance = 0.5 * BASE_PARTICLE_RADIUS;
                direction = random_shift;
            }
            let shift = ((2.0 * BASE_PARTICLE_RADIUS - distance) / (distance * 2.0))
                .min(2.0 * BASE_PARTICLE_RADIUS);
            // let shift = (2.0 * BASE_PARTICLE_RADIUS - distance) / 2.0;
            direction[0] += random_shift[0] * COLLISION_RANDOMNESS;
            direction[1] += random_shift[1] * COLLISION_RANDOMNESS;
            // if shift < 0.0 {
            //     continue;
            // }
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

            // self.particles[idxs[0]].velocity[0] += shift * direction[0] * 4.0;
            // self.particles[idxs[1]].velocity[0] -= shift * direction[0] * 4.0;
            // self.particles[idxs[0]].velocity[1] += shift * direction[1] * 4.0;
            // self.particles[idxs[1]].velocity[1] -= shift * direction[1] * 4.0;
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

        for _ in 0..NUM_PARTICLE_ITERS {
            self.self_collide_particles();
        }
    }

    fn particle_to_grid_velocity(&mut self) {
        self.x_velocity.0.0.clear();
        self.y_velocity.0.0.clear();

        for particle in &self.particles {
            self.x_velocity
                .particle_to_cell(particle.velocity[0], particle.pos);
            self.y_velocity
                .particle_to_cell(particle.velocity[1], particle.pos);
        }
    }

    fn make_incompressible(&mut self) {
        for pos in self.particle_grid.0.0.keys() {
            let neighbour_pos = [[0, -1], [-1, 0], [0, 0], [0, 0]].iter().map(|delta| {
                [
                    (pos[0] as i32 + delta[0]) as u32,
                    (pos[1] as i32 + delta[1]) as u32,
                ]
            });
            let is_neighbour_exist = neighbour_pos
                .clone()
                .map(|neighbour_pos| self.particle_grid.0.0.contains_key(&neighbour_pos))
                .map(|does_exist| if does_exist { 1.0 } else { 0.0 })
                .collect::<Vec<_>>();
            let neighbour_pos = neighbour_pos.collect::<Vec<_>>();

            let divergence = self
                .x_velocity
                .0
                .0
                .get(&neighbour_pos[0])
                .copied()
                .unwrap_or(0.0)
                * is_neighbour_exist[0]
                + self
                    .y_velocity
                    .0
                    .0
                    .get(&neighbour_pos[1])
                    .copied()
                    .unwrap_or(0.0)
                    * is_neighbour_exist[1]
                - self
                    .x_velocity
                    .0
                    .0
                    .get(&neighbour_pos[2])
                    .copied()
                    .unwrap_or(0.0)
                    * is_neighbour_exist[2]
                - self
                    .y_velocity
                    .0
                    .0
                    .get(&neighbour_pos[3])
                    .copied()
                    .unwrap_or(0.0)
                    * is_neighbour_exist[3];
            let total = is_neighbour_exist.iter().sum();
            if f64::is_subnormal(total) {
                continue;
            }

            if let Some(vel) = self.x_velocity.0.0.get_mut(&neighbour_pos[0]) {
                *vel -= divergence / total
            }
            if let Some(vel) = self.y_velocity.0.0.get_mut(&neighbour_pos[1]) {
                *vel -= divergence / total
            }
            if let Some(vel) = self.x_velocity.0.0.get_mut(&neighbour_pos[2]) {
                *vel += divergence / total
            }
            if let Some(vel) = self.y_velocity.0.0.get_mut(&neighbour_pos[3]) {
                *vel += divergence / total
            }
        }
    }

    fn grid_to_particle_velocity(&mut self) {
        for particle in &mut self.particles {
            particle.velocity[0] += self.x_velocity.grid_to_particle(particle.pos);
            particle.velocity[1] += self.y_velocity.grid_to_particle(particle.pos);
        }
    }

    fn simulate(&mut self, dt: f64) {
        self.simulate_particles(dt);
        self.particle_to_grid_velocity();
        for _ in 0..3 {
            self.make_incompressible();
        }
        self.grid_to_particle_velocity();
    }

    fn render_cell<G: Graphics>(&self, ctx: Context, graphics_buffer: &mut G, x: u32, y: u32) {
        let Some(cell) = self.y_velocity.0.0.get(&[x, y]) else {
            return;
        };

        rectangle(
            [1.0, 0.0, 0.0, *cell as f32 / 32.0],
            [
                (x as f64 - VelocityGrid::<Y>::OFFSET[0]) * CELL_SIZE,
                (y as f64 - VelocityGrid::<Y>::OFFSET[1]) * CELL_SIZE,
                CELL_SIZE,
                CELL_SIZE,
            ],
            ctx.transform,
            graphics_buffer,
        )
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
        for x in 0..=self.size[0] {
            for y in 0..=self.size[1] {
                self.render_cell(ctx, graphics_buffer, x, y)
            }
        }
    }

    fn debug(&self) {
        dbg!(&self.y_velocity);
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
    // simulation.spawn(Particle {
    //     pos: [100.0, 100.0],
    //     velocity: [0.0, 0.0],
    // });
    // simulation.spawn(Particle {
    //     pos: [100.0, 150.0],
    //     velocity: [0.0, 0.0],
    // });

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

        if let Event::Input(
            Input::Button(ButtonArgs {
                button: Button::Keyboard(Key::D),
                ..
            }),
            _,
        ) = event
        {
            simulation.debug();
        }
    }
}
