use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use piston::WindowSettings;
use piston_window::{ellipse::circle, *};

const BASE_PARTICLE_RADIUS: f64 = 10.0;
const CELL_SIZE: f64 = BASE_PARTICLE_RADIU * 2.0;

struct Particle {
    pos: [f64; 2],
    velocity: [f64; 2],
}

#[derive(Default)]
struct Grid {
    inner: HashMap<[u32; 2], f32>,
}

struct Simulation {
    particles: Vec<Particle>,
    x_velocity: Grid,
    y_velocity: Grid,
    pressure: Grid,
    size: [u32; 2],
}

impl Simulation {
    fn new(size: [u32; 2]) -> Simulation {
        return Simulation {
            particles: vec![],
            x_velocity: Grid::default(),
            y_velocity: Grid::default(),
            pressure: Grid::default(),
            size,
        };
    }

    fn simulate_particles(&mut self, dt: f64) {
        todo!()
    }

    fn particle_to_grid_velocity(&mut self) {
        todo!()
    }

    fn make_incompressible(&mut self) {
        todo!()
    }

    fn grid_to_particle_velocity(&mut self) {
        todo!()
    }

    fn simulate(&mut self, dt: f64) {
        self.simulate_particles(dt);
        self.particle_to_grid_velocity();
        self.make_incompressible();
        self.grid_to_particle_velocity();
    }

    fn render<G: Graphics>(&self, ctx: Context, graphics_buffer: &mut G) {
        todo!()
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Fluid simulation", [1000, 1000])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut simulation = Simulation::new([100, 100]);

    window.set_lazy(false);
    let mut prev_frame = SystemTime::now();
    while let Some(event) = window.next() {
        window.draw_2d(&event, |ctx, graphics_buffer, _device| {
            let dt = SystemTime::now()
                .duration_since(prev_frame)
                .expect("Time may have gone backwatds");
            simulation.simulate(dt.as_secs_f64());
            simulation.render(ctx, graphics_buffer);
        });
        prev_frame = SystemTime::now()
    }
}
