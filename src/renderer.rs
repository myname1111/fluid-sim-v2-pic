use std::sync::Arc;

use winit::window::Window;

use crate::simulation::Simulation;

pub struct SimulationRenderer {
    window: Arc<Window>,
}

impl SimulationRenderer {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self { window })
    }

    pub fn render(&mut self, simulation: &Simulation) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {}
}
