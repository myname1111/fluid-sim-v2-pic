use std::{collections::HashSet, marker::PhantomData, time::SystemTime};

use piston::WindowSettings;
use piston_window::{color::GREEN, ellipse::circle, *};

const BASE_PARTICLE_RADIUS: f64 = 10.0;
const CELL_SIZE: f64 = BASE_PARTICLE_RADIUS * 2.0;
const PARTICLE_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const GRAVITY: f64 = 9.8;
const NUM_PARTICLE_ITERS: usize = 10;
const COLLISION_RANDOMNESS: f64 = 0.1;
const DIVERGENCE_SOLVER_ITERS: usize = 20;
const OVERRELAXATION: f64 = 1.9;
const MIN: f64 = 0.04;

trait ProblematicallySmall {
    fn is_problematically_small(&self) -> bool;
}

impl ProblematicallySmall for f32 {
    fn is_problematically_small(&self) -> bool {
        self.is_subnormal() || *self == 0.0
    }
}

impl ProblematicallySmall for f64 {
    fn is_problematically_small(&self) -> bool {
        self.is_subnormal() || *self == 0.0
    }
}

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
        if (distance * 65536.0).is_problematically_small() {
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
struct Grid<T>(Vec<Option<T>>, [u32; 2]);

impl<T> Grid<T> {
    fn new(size: [u32; 2]) -> Self {
        let mut items = vec![];
        for _ in 0..(size[0] * size[1]) {
            items.push(None);
        }
        Self(items, size)
    }

    fn pos(&self, index: usize) -> [u32; 2] {
        [index as u32 % self.1[0], index as u32 / self.1[0]]
    }

    fn index(&self, pos: [u32; 2]) -> Option<usize> {
        if pos[0] >= self.1[0] {
            return None;
        }
        if pos[1] >= self.1[1] {
            return None;
        }
        Some((pos[1] * self.1[0] + pos[0]) as usize)
    }

    fn get(&self, pos: [u32; 2]) -> Option<&T> {
        self.0
            .get(self.index(pos)?)
            .and_then(|inner| inner.as_ref())
    }

    fn get_mut(&mut self, pos: [u32; 2]) -> Option<&mut Option<T>> {
        let index = self.index(pos)?;
        self.0.get_mut(index)
    }

    fn get_mut_filled(&mut self, pos: [u32; 2]) -> Option<&mut T> {
        self.get_mut(pos).and_then(|inner| inner.as_mut())
    }

    fn remove(&mut self, pos: [u32; 2]) {
        if let Some(cell) = self.get_mut(pos) {
            *cell = None;
        }
    }

    fn insert(&mut self, pos: [u32; 2], value: T) {
        if let Some(cell) = self.get_mut(pos) {
            *cell = Some(value);
        }
    }

    fn modify_or_insert(&mut self, pos: [u32; 2], insertion: T) -> Option<&mut T> {
        let cell = self.get_mut(pos)?;
        if cell.is_none() {
            *cell = Some(insertion);
        }
        cell.as_mut()
    }

    fn clear(&mut self) {
        for cell in &mut self.0 {
            *cell = None;
        }
    }
}

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
        let offset = Self::offset(pos).map(|pos| pos.clamp(MIN, 1.0 - MIN));

        [
            (1.0 - offset[0]) * (1.0 - offset[1]),
            offset[0] * (1.0 - offset[1]),
            (1.0 - offset[0]) * offset[1],
            offset[0] * offset[1],
        ]
    }
}

impl ParticleGrid {
    fn new(size: [u32; 2]) -> Self {
        Self(Grid::<_>::new(size))
    }

    fn move_particle_grid_pos(
        &mut self,
        old: [u32; 2],
        new: [u32; 2],
        particle_idx: usize,
    ) -> bool {
        let mut remove_old = false;

        if let Some(old_cell) = self.0.get_mut_filled(old) {
            old_cell.remove(&particle_idx);
            remove_old = old_cell.is_empty();
        }

        if remove_old {
            self.0.remove(old);
        }

        if let Some(new_cell) = self.0.get_mut_filled(new) {
            new_cell.insert(particle_idx);
        } else {
            let mut new_set = HashSet::new();
            new_set.insert(particle_idx);
            self.0.insert(new, new_set);
        }

        remove_old
    }

    fn is_wall(size: [u32; 2], pos: [u32; 2]) -> bool {
        let is_x = pos[0] == (size[0] - 1) || pos[0] == 0;
        let is_y = pos[1] == (size[1] - 1) || pos[1] == 0;
        is_x || is_y
    }
}

impl GridParticleInterface for ParticleGrid {
    const OFFSET: [f64; 2] = [0.0, 0.0];
}

trait PosDirection {
    const INDEX: usize;
}

#[derive(Clone, Copy, Default, Debug)]
struct X;

impl PosDirection for X {
    const INDEX: usize = 0;
}

#[derive(Clone, Copy, Default, Debug)]
struct Y;

impl PosDirection for Y {
    const INDEX: usize = 1;
}

#[derive(Default, Debug)]
struct VelocityGrid<P: PosDirection>(Grid<f64>, PhantomData<P>);

impl<P: PosDirection> VelocityGrid<P> {
    fn new(size: [u32; 2]) -> Self {
        Self(Grid::<f64>::new(size), PhantomData)
    }
}

impl<P: PosDirection> VelocityGrid<P>
where
    VelocityGrid<P>: GridParticleInterface + DefinedPositionsRetrivable,
{
    fn particle_to_cell(
        &mut self,
        velocity: f64,
        pos: [f64; 2],
        weights_grid: &mut Grid<f64>,
        particle_grid: &ParticleGrid,
    ) {
        let grid_pos = Self::get_grid_pos(pos);
        // dbg!(pos, grid_pos);
        let neighbours = [
            [grid_pos[0], grid_pos[1]],
            [grid_pos[0] + 1, grid_pos[1]],
            [grid_pos[0], grid_pos[1] + 1],
            [grid_pos[0] + 1, grid_pos[1] + 1],
        ]
        .into_iter()
        .filter(|neighbour_pos| Self::is_position_defined(neighbour_pos, particle_grid));
        let weights = Self::get_weights(pos);
        for (neighbour, weight) in neighbours.zip(weights.iter()) {
            let Some(vel) = self.0.modify_or_insert(neighbour, 0.0) else {
                // dbg!(neighbour);
                continue;
            };
            let Some(weight_sum) = weights_grid.modify_or_insert(neighbour, 0.0) else {
                continue;
            };
            *vel += weight * velocity;
            *weight_sum += weight;
        }
    }

    fn paricles_to_grid(&mut self, particles: &[Particle], grid: &ParticleGrid) {
        let mut weights_grid = Grid::<f64>::new(self.0.1);
        for particle in particles {
            self.particle_to_cell(
                particle.velocity[P::INDEX],
                particle.pos,
                &mut weights_grid,
                grid,
            );
        }

        // dbg!(self.0.1);
        for (idx, velocity) in self.0.0.iter_mut().enumerate() {
            let Some(velocity) = velocity else {
                continue;
            };
            let weight = weights_grid.0[idx].unwrap_or(1.0);
            if weight.is_problematically_small() {
                continue;
            }
            // dbg!(weight, idx);
            *velocity /= weight
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
        let mut total_weights = 0.0;

        for (neighbour_pos, weight) in neighbours.iter().zip(weights.iter()) {
            let Some(neighbour) = self.0.get(*neighbour_pos) else {
                continue;
            };
            total += *neighbour * *weight;
            total_weights += *weight;
        }
        total / total_weights
    }

    fn is_position_defined(vel_pos: &[u32; 2], particle_grid: &ParticleGrid) -> bool {
        let Some(positions_to_check_for) = Self::defined_positions(vel_pos) else {
            return false;
        };
        let mut out = true;
        for pos in positions_to_check_for {
            let is_air = particle_grid
                .0
                .get(pos)
                .map(|cell| cell.is_empty())
                .unwrap_or(false);
            out &= !is_air
        }
        out
    }
}

impl GridParticleInterface for VelocityGrid<X> {
    const OFFSET: [f64; 2] = [0.0, -0.5];
}

impl GridParticleInterface for VelocityGrid<Y> {
    const OFFSET: [f64; 2] = [-0.5, 0.0];
}

trait DefinedPositionsRetrivable {
    fn defined_positions(vel_pos: &[u32; 2]) -> Option<[[u32; 2]; 2]>;
}

impl DefinedPositionsRetrivable for VelocityGrid<X> {
    fn defined_positions(vel_pos: &[u32; 2]) -> Option<[[u32; 2]; 2]> {
        if vel_pos[0] == 0 {
            return None;
        }

        Some([*vel_pos, [vel_pos[0] - 1, vel_pos[1]]])
    }
}

impl DefinedPositionsRetrivable for VelocityGrid<Y> {
    fn defined_positions(vel_pos: &[u32; 2]) -> Option<[[u32; 2]; 2]> {
        if vel_pos[1] == 0 {
            return None;
        }

        Some([*vel_pos, [vel_pos[0], vel_pos[1] - 1]])
    }
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
            x_velocity: VelocityGrid::<X>::new([size[0] + 1, size[1]]),
            y_velocity: VelocityGrid::<Y>::new([size[0], size[1] + 1]),
            particle_grid: ParticleGrid::new(size),
            size,
        }
    }

    fn spawn(&mut self, particle: Particle) {
        self.particles.push(particle);
        let new_index = self.particles.len() - 1;
        let grid_pos = ParticleGrid::get_grid_pos(particle.pos);
        if let Some(hashset) = self.particle_grid.0.get_mut_filled(grid_pos) {
            hashset.insert(new_index);
        } else {
            let mut new_hashset = HashSet::new();
            new_hashset.insert(new_index);
            self.particle_grid.0.insert(grid_pos, new_hashset);
        }
    }

    fn get_particle_collision(&self) -> HashSet<[usize; 2]> {
        let mut collisions = HashSet::new();

        for (idx, particles) in self.particle_grid.0.0.iter().enumerate() {
            let Some(particles) = particles else {
                continue;
            };
            let pos = self.particle_grid.0.pos(idx);
            let other_particles = (-1i32..=1i32)
                .flat_map(|x| (-1i32..=1i32).map(move |y| [x, y]))
                .map(|delta_pos| {
                    [
                        (pos[0] as i32 + delta_pos[0]) as u32,
                        (pos[1] as i32 + delta_pos[1]) as u32,
                    ]
                })
                .filter_map(|neighbour_pos| self.particle_grid.0.get(neighbour_pos))
                .flatten()
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
            if distance.is_problematically_small() {
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
        self.x_velocity.0.clear();
        self.y_velocity.0.clear();

        self.x_velocity
            .paricles_to_grid(&self.particles, &self.particle_grid);
        self.y_velocity
            .paricles_to_grid(&self.particles, &self.particle_grid);
    }

    fn make_incompressible(&mut self) {
        for (idx, particles) in self.particle_grid.0.0.iter().enumerate() {
            if particles.is_none() {
                continue;
            }
            let pos = self.particle_grid.0.pos(idx);
            let neighbour_pos = [[0, 0], [0, 0], [1, 0], [0, 1]].iter().map(|delta| {
                [
                    (pos[0] as i32 + delta[0]) as u32,
                    (pos[1] as i32 + delta[1]) as u32,
                ]
            });
            let is_wall = neighbour_pos
                .clone()
                .map(|pos| ParticleGrid::is_wall(self.size, pos))
                .collect::<Vec<_>>();
            let mask = is_wall
                .iter()
                .map(|is_wall| if *is_wall { 0.0 } else { 1.0 })
                .collect::<Vec<_>>();
            let neighbour_pos = neighbour_pos.collect::<Vec<_>>();
            // dbg!(self.y_velocity.0.0.get(&[2, 2]));

            let divergence = self
                .x_velocity
                .0
                .get(neighbour_pos[0])
                .copied()
                .unwrap_or(0.0)
                * mask[0]
                + self
                    .y_velocity
                    .0
                    .get(neighbour_pos[1])
                    .copied()
                    .unwrap_or(0.0)
                    * mask[1]
                - self
                    .x_velocity
                    .0
                    .get(neighbour_pos[2])
                    .copied()
                    .unwrap_or(0.0)
                    * mask[2]
                - self
                    .y_velocity
                    .0
                    .get(neighbour_pos[3])
                    .copied()
                    .unwrap_or(0.0)
                    * mask[3];
            let divergence = divergence * OVERRELAXATION;
            let total = mask.iter().sum::<f64>();
            if total.is_problematically_small() {
                continue;
            }

            // if total < 4.0 {
            //     dbg!(divergence, pos);
            // }
            for (idx, pos) in neighbour_pos.iter().enumerate() {
                if is_wall[idx] {
                    continue;
                }
                let vel_grid = if idx % 2 == 0 {
                    &mut self.x_velocity.0
                } else {
                    &mut self.y_velocity.0
                };
                let sign = ((idx as i32 / 2) * 2 - 1) as f64;
                let Some(vel) = vel_grid.modify_or_insert(*pos, 0.0) else {
                    continue;
                };
                *vel += sign * divergence / total
            }
        }
    }

    fn grid_to_particle_velocity(&mut self) {
        for particle in &mut self.particles {
            particle.velocity[0] = self.x_velocity.grid_to_particle(particle.pos);
            particle.velocity[1] = self.y_velocity.grid_to_particle(particle.pos);
        }
    }

    fn simulate(&mut self, dt: f64) {
        self.simulate_particles(dt);
        self.particle_to_grid_velocity();
        for _ in 0..DIVERGENCE_SOLVER_ITERS {
            self.make_incompressible();
        }
        // dbg!(&self.particles.first());
        self.grid_to_particle_velocity();
    }

    fn render_cell<G: Graphics>(&self, ctx: Context, graphics_buffer: &mut G, x: u32, y: u32) {
        let top = self.y_velocity.0.get([x, y]).cloned().unwrap_or(0.0);
        let bottom = self.y_velocity.0.get([x, y + 1]).cloned().unwrap_or(0.0);
        let right = self.x_velocity.0.get([x, y]).cloned().unwrap_or(0.0);
        let left = self.x_velocity.0.get([x + 1, y]).cloned().unwrap_or(0.0);

        let y_vel = (top + bottom) / 2.0;
        let x_vel = (left + right) / 2.0;

        let pos = [(x as f64) * CELL_SIZE, (y as f64) * CELL_SIZE];
        let pos_delta = [pos[0] + x_vel / 5.0, pos[1] + y_vel / 5.0];

        line::Line {
            color: GREEN,
            radius: 0.5,
            shape: line::Shape::Square,
        }
        .draw_arrow(
            [pos[0], pos[1], pos_delta[0], pos_delta[1]],
            4.0,
            &ctx.draw_state,
            ctx.transform,
            graphics_buffer,
        );
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
        // for x in 0..self.size[0] {
        //     for y in 0..self.size[1] {
        //         self.render_cell(ctx, graphics_buffer, x, y)
        //     }
        // }
    }

    fn debug(&self) {
        dbg!(&self.particles.first());
        // dbg!(&self.x_velocity);
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Fluid simulation", [1000, 1000])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut simulation = Simulation::new([60, 40]);
    // simulation.y_velocity.0.insert([2, 2], 100.0);
    for x in 0..10 {
        for y in 0..10 {
            simulation.spawn(Particle {
                pos: [
                    100.0 + x as f64 * BASE_PARTICLE_RADIUS * 2.0,
                    100.0 + y as f64 * BASE_PARTICLE_RADIUS * 2.0,
                ],
                velocity: [100.0, 0.0],
            });
        }
    }
    // simulation.spawn(Particle {
    //     pos: [80.0, 780.0],
    //     velocity: [0.0, 100.0],
    // });
    // simulation.spawn(Particle {
    //     pos: [80.0, 80.0],
    //     velocity: [0.0, 100.0],
    // });
    // simulation.spawn(Particle {
    //     pos: [130.0, 800.0],
    //     velocity: [100.0, 0.0],
    // });

    window.set_lazy(false);
    let mut prev_frame = SystemTime::now();
    let mut frame_idx = 0;
    let mut total_time = 0.0;
    while let Some(event) = window.next() {
        window.draw_2d(&event, |ctx, graphics_buffer, _device| {
            let dt = SystemTime::now()
                .duration_since(prev_frame)
                .expect("Time may have gone backwatds");
            total_time += dt.as_secs_f64();
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

        frame_idx += 1;
    }

    println!("Average frame rate {}", frame_idx as f64 / total_time)
}
