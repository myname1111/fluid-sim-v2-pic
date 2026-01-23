mod renderer;
mod simulation;

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use simulation::{Particle, Simulation};
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{renderer::SimulationRenderer, simulation::BASE_PARTICLE_RADIUS};

struct App {
    renderer: Option<SimulationRenderer>,
    simulation: Simulation,
    prev_time: SystemTime,
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.renderer = Some(pollster::block_on(SimulationRenderer::new(window)).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let dt = SystemTime::now().duration_since(self.prev_time).unwrap();
                self.prev_time = SystemTime::now();
                // dbg!(dt);

                self.simulation.simulate(dt.as_secs_f64());
                match renderer.render(&self.simulation) {
                    Ok(_) => (),
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        renderer.recreate_window()
                    }
                    Err(e) => log::error!("Error detected! {e}"),
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => renderer.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}

fn main() -> anyhow::Result<()> {
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

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App {
        renderer: None,
        simulation,
        prev_time: SystemTime::now(),
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
