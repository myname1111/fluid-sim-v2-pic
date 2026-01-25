use std::{collections::HashSet, marker::PhantomData};

pub const BASE_PARTICLE_RADIUS: f64 = 10.0;
pub const CELL_SIZE: f64 = BASE_PARTICLE_RADIUS * 2.0;
pub const GRAVITY: f64 = 9.8;
pub const NUM_PARTICLE_ITERS: usize = 2;
pub const COLLISION_RANDOMNESS: f64 = 0.1;
pub const DIVERGENCE_SOLVER_ITERS: usize = 20;
pub const OVERRELAXATION: f64 = 1.9;
pub const MIN: f64 = 0.04;
pub const REST_DENSITY: f64 = 1.0;
pub const STIFFNESS: f64 = 10.0;

const NEIGHBOUR_KERNEL: [[i32; 2]; 8] = [
    [-1, -1],
    [-1, 0],
    [-1, 1],
    [0, -1],
    [0, 1],
    [1, -1],
    [1, 0],
    [1, 1],
];

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
pub struct Particle {
    pub pos: [f64; 2],
    pub velocity: [f64; 2],
}

impl Particle {
    fn push_out_of_border(&mut self, size: [f64; 2]) {
        if self.pos[0] > size[0] {
            self.velocity[0] = self.velocity[0].min(0.0);
        }

        if self.pos[0] <= 0.0 {
            self.velocity[0] = self.velocity[0].max(0.0);
        }

        if self.pos[1] > size[1] {
            self.velocity[1] = self.velocity[1].min(0.0);
        }

        if self.pos[1] <= 0.0 {
            self.velocity[1] = self.velocity[1].max(0.0);
        }

        self.pos[0] = self.pos[0].clamp(0.01, size[0] - 0.01);
        self.pos[1] = self.pos[1].clamp(0.01, size[1] - 0.01);
    }

    fn simulate(&mut self, dt: f64) {
        self.pos[0] += self.velocity[0] * dt;
        self.pos[1] += self.velocity[1] * dt;
    }
}

#[derive(Default, Debug)]
pub struct Grid<T>(pub Vec<Option<T>>, pub [u32; 2]);

impl<T> Grid<T> {
    pub fn new(size: [u32; 2]) -> Self {
        let mut items = vec![];
        for _ in 0..(size[0] * size[1]) {
            items.push(None);
        }
        Self(items, size)
    }

    pub fn pos(&self, index: usize) -> [u32; 2] {
        [index as u32 % self.1[0], index as u32 / self.1[0]]
    }

    pub fn index(&self, pos: [u32; 2]) -> Option<usize> {
        if pos[0] >= self.1[0] {
            return None;
        }
        if pos[1] >= self.1[1] {
            return None;
        }
        Some((pos[1] * self.1[0] + pos[0]) as usize)
    }

    pub fn get(&self, pos: [u32; 2]) -> Option<&T> {
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
pub struct Cell(pub Vec<usize>);

impl Cell {
    fn remove(&mut self, key: usize) {
        let mut found_idx = None;
        for (idx, item) in self.0.iter().enumerate() {
            if *item == key {
                found_idx = Some(idx);
            }
        }
        let Some(idx) = found_idx else {
            return;
        };
        self.0.remove(idx);
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn insert(&mut self, item: usize) {
        self.0.push(item);
    }
}

#[derive(Default, Debug)]
pub struct ParticleGrid(pub Grid<Cell>);

pub trait GridParticleInterface {
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
            old_cell.remove(particle_idx);
            remove_old = old_cell.is_empty();
        }

        if remove_old {
            self.0.remove(old);
        }

        if let Some(new_cell) = self.0.get_mut_filled(new) {
            new_cell.insert(particle_idx);
        } else {
            let new_set = Cell(vec![particle_idx]);
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

pub trait PosDirection {
    const INDEX: usize;
}

#[derive(Clone, Copy, Default, Debug)]
pub struct X;

impl PosDirection for X {
    const INDEX: usize = 0;
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Y;

impl PosDirection for Y {
    const INDEX: usize = 1;
}

#[derive(Default, Debug)]
pub struct VelocityGrid<P: PosDirection>(pub Grid<f64>, pub PhantomData<P>);

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
        // dbg!(grid_pos, pos);
        let neighbours = [
            [grid_pos[0], grid_pos[1]],
            [grid_pos[0] + 1, grid_pos[1]],
            [grid_pos[0], grid_pos[1] + 1],
            [grid_pos[0] + 1, grid_pos[1] + 1],
        ]
        .into_iter()
        .filter(|neighbour_pos| Self::is_position_defined(neighbour_pos, particle_grid));
        // dbg!(neighbours.clone().collect::<Vec<_>>());
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
            return !particle_grid
                .0
                .get(*vel_pos)
                .map(Cell::is_empty)
                .unwrap_or(false);
        };
        let mut out = false;
        for pos in positions_to_check_for {
            let is_air = particle_grid
                .0
                .get(pos)
                .map(|cell| cell.is_empty())
                .unwrap_or(false);
            out |= !is_air
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

pub struct ParticleDensityGrid(pub Grid<f64>);

impl ParticleDensityGrid {
    fn new(size: [u32; 2]) -> Self {
        Self(Grid::new(size))
    }

    fn particle_to_cell(&mut self, pos: [f64; 2]) {
        let grid_pos = Self::get_grid_pos(pos);
        let neighbours = [
            [grid_pos[0], grid_pos[1]],
            [grid_pos[0] + 1, grid_pos[1]],
            [grid_pos[0], grid_pos[1] + 1],
            [grid_pos[0] + 1, grid_pos[1] + 1],
        ];
        let weights = Self::get_weights(pos);
        for (neighbour, weight) in neighbours.iter().zip(weights) {
            let Some(density) = self.0.modify_or_insert(*neighbour, 0.0) else {
                continue;
            };
            *density += weight;
        }
    }
}

impl GridParticleInterface for ParticleDensityGrid {
    const OFFSET: [f64; 2] = [-0.5, -0.5];
}

pub struct Simulation {
    particles: Vec<Particle>,
    x_velocity: VelocityGrid<X>,
    y_velocity: VelocityGrid<Y>,
    particle_grid: ParticleGrid,
    particle_density_grid: ParticleDensityGrid,
    size: [u32; 2],
}

impl Simulation {
    pub fn new(size: [u32; 2]) -> Simulation {
        Simulation {
            particles: vec![],
            x_velocity: VelocityGrid::<X>::new([size[0] + 1, size[1] + 1]),
            y_velocity: VelocityGrid::<Y>::new([size[0] + 1, size[1] + 1]),
            particle_grid: ParticleGrid::new(size),
            particle_density_grid: ParticleDensityGrid::new([size[0] + 1, size[1] + 1]),
            size,
        }
    }

    pub fn spawn(&mut self, particle: Particle) {
        self.particles.push(particle);
        let new_index = self.particles.len() - 1;
        let grid_pos = ParticleGrid::get_grid_pos(particle.pos);
        if let Some(hashset) = self.particle_grid.0.get_mut_filled(grid_pos) {
            hashset.insert(new_index);
        } else {
            let new_hashset = Cell(vec![new_index]);
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
            let other_particles = NEIGHBOUR_KERNEL
                .iter()
                .map(|delta_pos| {
                    [
                        (pos[0] as i32 + delta_pos[0]) as u32,
                        (pos[1] as i32 + delta_pos[1]) as u32,
                    ]
                })
                .filter_map(|neighbour_pos| self.particle_grid.0.get(neighbour_pos))
                .flat_map(|cell| &cell.0);

            for particle_idx in &particles.0 {
                for other_particle_idx in other_particles.clone() {
                    if particle_idx == other_particle_idx {
                        continue;
                    };

                    let key = if particle_idx < other_particle_idx {
                        [*other_particle_idx, *particle_idx]
                    } else {
                        [*particle_idx, *other_particle_idx]
                    };

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

    fn self_collide_particles(&mut self, size: [f64; 2]) {
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

            self.particles[idxs[0]].push_out_of_border(size);
            self.particles[idxs[1]].push_out_of_border(size);

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
        let size = [
            self.size[0] as f64 * CELL_SIZE,
            self.size[1] as f64 * CELL_SIZE,
        ];
        for (idx, particle) in self.particles.iter_mut().enumerate() {
            let old_grid_pos = ParticleGrid::get_grid_pos(particle.pos);

            particle.simulate(dt);
            particle.push_out_of_border(size);

            let new_grid_pos = ParticleGrid::get_grid_pos(particle.pos);

            if old_grid_pos == new_grid_pos {
                continue;
            }

            self.particle_grid
                .move_particle_grid_pos(old_grid_pos, new_grid_pos, idx);
        }

        for _ in 0..NUM_PARTICLE_ITERS {
            self.self_collide_particles(size);
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

    fn add_forces(&mut self, dt: f64) {
        for vel in &mut self.y_velocity.0.0 {
            let Some(vel) = vel else {
                continue;
            };
            *vel += GRAVITY * dt
        }
    }

    fn update_particle_density(&mut self) {
        self.particle_density_grid.0.clear();

        for particle in &self.particles {
            self.particle_density_grid.particle_to_cell(particle.pos);
        }
    }

    fn make_incompressible(&mut self) {
        for (idx, particles) in self.particle_grid.0.0.iter().enumerate() {
            if particles.is_none() {
                continue;
            }
            let pos = self.particle_grid.0.pos(idx);
            let neighbour_pos = [[0, 0], [0, 0], [1, 0], [0, 1]].map(|delta| {
                [
                    (pos[0] as i32 + delta[0]) as u32,
                    (pos[1] as i32 + delta[1]) as u32,
                ]
            });
            let is_wall = neighbour_pos.map(|pos| ParticleGrid::is_wall(self.size, pos));
            let mask = is_wall.map(|is_wall| if is_wall { 0.0 } else { 1.0 });
            // dbg!(self.y_velocity.0.index([5, 0]));

            let divergence = -self
                .x_velocity
                .0
                .get(neighbour_pos[0])
                .copied()
                .expect(&format!("{:?}", pos))
                * mask[0]
                - self
                    .y_velocity
                    .0
                    .get(neighbour_pos[1])
                    .copied()
                    .expect(&format!("{:?}", pos))
                    * mask[1]
                + self
                    .x_velocity
                    .0
                    .get(neighbour_pos[2])
                    .copied()
                    .expect(&format!("{:?}", pos))
                    * mask[2]
                + self
                    .y_velocity
                    .0
                    .get(neighbour_pos[3])
                    .copied()
                    .expect(&format!("{:?}", pos))
                    * mask[3];
            let density = self
                .particle_density_grid
                .0
                .get(pos)
                .copied()
                .unwrap_or(REST_DENSITY);
            // dbg!(density);
            let divergence = divergence * OVERRELAXATION - STIFFNESS * (density - REST_DENSITY);
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
                let sign = (1 - (idx as i32 / 2) * 2) as f64;
                let Some(vel) = vel_grid.modify_or_insert(*pos, 0.0) else {
                    continue;
                };
                *vel += sign * divergence * mask[idx] / total
            }
        }
    }

    fn grid_to_particle_velocity(&mut self) {
        for particle in &mut self.particles {
            particle.velocity[0] = self.x_velocity.grid_to_particle(particle.pos);
            particle.velocity[1] = self.y_velocity.grid_to_particle(particle.pos);
        }
    }

    pub fn simulate(&mut self, dt: f64) {
        self.simulate_particles(dt);
        self.update_particle_density();
        self.particle_to_grid_velocity();
        self.add_forces(dt);
        for _ in 0..DIVERGENCE_SOLVER_ITERS {
            self.make_incompressible();
        }
        self.grid_to_particle_velocity();
    }

    pub fn debug(&self) {
        dbg!(&self.particle_grid);
        // dbg!(&self.x_velocity);
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn x_velocity(&self) -> &VelocityGrid<X> {
        &self.x_velocity
    }

    pub fn y_velocity(&self) -> &VelocityGrid<Y> {
        &self.y_velocity
    }

    pub fn particle_density_grid(&self) -> &ParticleDensityGrid {
        &self.particle_density_grid
    }
}
