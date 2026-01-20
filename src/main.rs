mod renderer;
mod simulation;

use std::time::SystemTime;

use simulation::{Particle, Simulation};

use crate::{renderer::SimulationRenderer, simulation::BASE_PARTICLE_RADIUS};

fn main() {
    env_logger::init();
    let mut simulation = Simulation::new([60, 40]);
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
    //     pos: [80.0, 760.0],
    //     velocity: [0.0, 100.0],
    // });
    // simulation.spawn(Particle {
    //     pos: [100.0, 780.0],
    //     velocity: [0.0, 0.0],
    // });
    // simulation.spawn(Particle {
    //     pos: [130.0, 800.0],
    //     velocity: [100.0, 0.0],
    // });
}
