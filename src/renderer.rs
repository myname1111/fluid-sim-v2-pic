use piston_window::{Context, Graphics, color::GREEN, ellipse, ellipse::circle, line};

use crate::simulation::{BASE_PARTICLE_RADIUS, CELL_SIZE, Simulation};

pub const PARTICLE_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

pub struct SimulationRenderer<'a, G: Graphics> {
    ctx: &'a Context,
    graphics_buffer: &'a mut G,
}

impl<'a, G: Graphics> SimulationRenderer<'a, G> {
    pub fn new(ctx: &'a Context, graphics_buffer: &'a mut G) -> Self {
        Self {
            ctx,
            graphics_buffer,
        }
    }

    fn render_cell(&mut self, simulation: &Simulation, x: u32, y: u32) {
        let top = simulation
            .y_velocity()
            .0
            .get([x, y])
            .cloned()
            .unwrap_or(0.0);
        let bottom = simulation
            .y_velocity()
            .0
            .get([x, y + 1])
            .cloned()
            .unwrap_or(0.0);
        let right = simulation
            .x_velocity()
            .0
            .get([x, y])
            .cloned()
            .unwrap_or(0.0);
        let left = simulation
            .x_velocity()
            .0
            .get([x + 1, y])
            .cloned()
            .unwrap_or(0.0);

        let y_vel = (top + bottom) / 2.0;
        let x_vel = (left + right) / 2.0;

        let pos = [(x as f64 + 0.5) * CELL_SIZE, (y as f64 + 0.5) * CELL_SIZE];
        let pos_delta = [pos[0] + x_vel / 5.0, pos[1] + y_vel / 5.0];

        line::Line {
            color: GREEN,
            radius: 0.5,
            shape: line::Shape::Square,
        }
        .draw_arrow(
            [pos[0], pos[1], pos_delta[0], pos_delta[1]],
            4.0,
            &self.ctx.draw_state,
            self.ctx.transform,
            self.graphics_buffer,
        );
    }

    pub fn render(&mut self, simulation: &Simulation) {
        for particle in simulation.particles() {
            ellipse(
                PARTICLE_COLOR,
                circle(particle.pos[0], particle.pos[1], BASE_PARTICLE_RADIUS),
                self.ctx.transform,
                self.graphics_buffer,
            );
        }
        // for x in 0..self.size[0] {
        //     for y in 0..self.size[1] {
        //         self.render_cell(ctx, graphics_buffer, x, y)
        //     }
        // }
    }
}
