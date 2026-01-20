mod renderer;
mod simulation;

use std::time::SystemTime;

use piston::{Button, ButtonArgs, Event, Input, Key, WindowSettings};
use piston_window::{EventLoop, Graphics, PistonWindow};
use simulation::{Particle, Simulation};

use crate::{renderer::SimulationRenderer, simulation::BASE_PARTICLE_RADIUS};

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Fluid simulation", [1000, 1000])
        .exit_on_esc(true)
        .build()
        .unwrap();

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

    window.set_lazy(false);
    let mut prev_frame = SystemTime::now();
    let mut frame_idx = 0;
    let mut total_time = 0.0;
    window.events.max_fps(60);
    while let Some(event) = window.next() {
        window.draw_2d(&event, |ctx, graphics_buffer, _device| {
            let dt = SystemTime::now()
                .duration_since(prev_frame)
                .expect("Time may have gone backwatds");
            total_time += dt.as_secs_f64();
            prev_frame = SystemTime::now();
            // simulation.simulate(dt.as_secs_f64());
            graphics_buffer.clear_color([1.0, 1.0, 1.0, 1.0]);

            // let mut simulation_renderer = SimulationRenderer::new(&ctx, graphics_buffer);
            // simulation_renderer.render(&simulation);
            frame_idx += 1;
            // dbg!(dt);
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

    println!("Average frame rate {}", frame_idx as f64 / total_time)
}
