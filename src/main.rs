#![deny(clippy::all)]
#![forbid(unsafe_code)]

use crate::gui::Framework;
use cell::{CellState, InputState, RuleState, Rules};
use error_iter::ErrorIter as _;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod gui;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const BOX_SIZE: i16 = 64;
// Convenience variable so we don't have to use `as usize` to "cast" the `u32`
// (4 bytes) to a `usize` (4 bytes on a 64-bit system, but the actual length
// it varies depending on the architecture you're compiling for) whenever we
// want to use the height or width with array indices (which are usually
// represented with `usize`)
const WIDTH_USIZE: usize = crate::WIDTH as usize;
const HEIGHT_USIZE: usize = crate::HEIGHT as usize;

mod cell {
    use std::collections::HashMap;

    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    pub enum CellState {
        On,
        Off,
    }

    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    pub struct RuleState(pub u32);

    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    pub struct InputState(pub [CellState; 9]);

    pub struct Rule {
        input: InputState,
        output: CellState,
    }
    pub type Rules = HashMap<InputState, CellState>;

    impl From<u32> for Rule {
        fn from(rule: u32) -> Self {
            let mut cell_states = [CellState::Off; 9];
            for i in 0..cell_states.len() {
                if (0b1 << i) & rule != 0 {
                    cell_states[i] = CellState::On;
                }
            }
            let output = if (0b1 << 9) & rule != 0 {
                CellState::On
            } else {
                CellState::Off
            };
            Rule {
                input: InputState(cell_states),
                output,
            }
        }
    }
}
/// Representation of the application state. In this example, a box will bounce around the screen.
struct World {
    rows: [[cell::CellState; WIDTH_USIZE]; HEIGHT_USIZE],
    rules: cell::Rules,
    generation: u32,
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Pixels + egui")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let (mut pixels, mut framework) = {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture)?;
        let framework = Framework::new(
            &event_loop,
            window_size.width,
            window_size.height,
            scale_factor,
            &pixels,
        );

        (pixels, framework)
    };
    let mut world = World::new();

    event_loop.run(move |event, _, control_flow| {
        if let Some(new_rule) = framework.get_rule() {
            world.rules = randomize_rules(new_rule as u64);
            for i in 0..HEIGHT_USIZE {
                for j in 0..WIDTH_USIZE {
                    world.rows[i][j] = if rand::random::<f32>() < 0.1 {
                        CellState::On
                    } else {
                        CellState::Off
                    };
                }
            }

            framework.clear_new_rule();
        }
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Update the scale factor
            if let Some(scale_factor) = input.scale_factor() {
                framework.scale_factor(scale_factor);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                framework.resize(size.width, size.height);
            }

            // if input.key_pressed(VirtualKeyCode::U) {
            //     world.rules = randomize_rules();
            // }
            if input.key_pressed(VirtualKeyCode::K) {
                for i in 0..HEIGHT_USIZE {
                    for j in 0..WIDTH_USIZE {
                        world.rows[i][j] = CellState::Off;
                    }
                }
                // world.rows[rand::random::<usize>() % HEIGHT_USIZE]
                //     [rand::random::<usize>() % WIDTH_USIZE] = PixelState::On;
                return;
            }

            if input.key_pressed(VirtualKeyCode::R) {
                for i in 0..HEIGHT_USIZE {
                    for j in 0..WIDTH_USIZE {
                        world.rows[i][j] = if rand::random() {
                            CellState::On
                        } else {
                            CellState::Off
                        }
                    }
                }
                // world.rows[rand::random::<usize>() % HEIGHT_USIZE]
                //     [rand::random::<usize>() % WIDTH_USIZE] = PixelState::On;
                return;
            }

            // Update internal state and request a redraw
            world.update();
            window.request_redraw();
        }

        match event {
            Event::WindowEvent { event, .. } => {
                // Update egui inputs
                framework.handle_event(&event);
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                // Draw the world
                world.draw(pixels.frame_mut());

                // Prepare egui
                framework.prepare(&window);

                // Render everything together
                let render_result = pixels.render_with(|encoder, render_target, context| {
                    // Render the world texture
                    context.scaling_renderer.render(encoder, render_target);

                    // Render egui
                    framework.render(encoder, render_target, context);

                    Ok(())
                });

                // Basic error handling
                if let Err(err) = render_result {
                    log_error("pixels.render", err);
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => (),
        }
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

fn randomize_rules(seed: u64) -> Rules {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut rules = Rules::with_capacity(362880);
    for i in 0..362880_u32 {
        let rule = if rng.gen::<f32>() < 0.1 {
            CellState::On
        } else {
            CellState::Off
        };
        rules.insert(i.into(), rule);
    }
    rules
}

impl World {
    fn new() -> Self {
        let rules = randomize_rules(0);

        let mut default = Self {
            rows: [[CellState::Off; WIDTH_USIZE]; HEIGHT_USIZE],
            rules,
            generation: 0,
        };

        for i in 0..WIDTH_USIZE {
            default.rows[0][i] = if rand::random() {
                CellState::On
            } else {
                CellState::Off
            };
        }

        default
    }

    fn get_distribution(&self) -> (f32, f32) {
        let mut on = 0;
        let mut off = 0;
        for i in 0..(HEIGHT_USIZE) {
            for j in 0..WIDTH_USIZE {
                match self.rows[i][j] {
                    CellState::On => {
                        on += 1;
                    }
                    CellState::Off => {
                        off += 1;
                    }
                }
            }
        }

        (
            (on as f32 / ((WIDTH_USIZE * HEIGHT_USIZE) as f32)) * 100.0,
            (off as f32 / ((WIDTH_USIZE * HEIGHT_USIZE) as f32)) * 100.0,
        )
    }

    fn update(&mut self) {
        self.generation += 1;
        // let (on, off) = self.get_distribution();
        // println!(
        //     "Generation: {}\t\tOn: {}%\tOff: {}%",
        //     self.generation, on_string, off_string
        // );
        // if self.generation > 100 && !(on < 0.1 || off < 0.1) {
        //     self.generation = 0;
        //     self.rules = randomize_rules();
        //     println!("Nothing interesting going on, resetting!");
        // }
        let mut next_state = [[CellState::Off; WIDTH_USIZE]; HEIGHT_USIZE];
        for i in 0..(WIDTH_USIZE * HEIGHT_USIZE) {
            let row = i / WIDTH_USIZE;
            let col = i % WIDTH_USIZE;

            // Calculate the X index for neighbors on the left. If the current
            // cell is at X=0, then look at the right-most column to treat the
            // row as if it wraps around.
            let left_neighbor_x_index = if col == 0 { WIDTH_USIZE - 1 } else { col - 1 };

            // X index for neighbors to the right. If the current cell is on the
            // right-most index, we "wrap around" and look at the left-most (0)
            // index.
            let right_neighbor_x_index = if col == WIDTH_USIZE - 1 { 0 } else { col + 1 };

            // Calculate the Y index for neighbors above the current row. If the
            // current row is 0, we "wrap around" to the bottom row
            let upper_neighbor_y_index = if row == 0 { HEIGHT_USIZE - 1 } else { row - 1 };

            let lower_neighbor_y_index = if row == HEIGHT_USIZE - 1 { 0 } else { row + 1 };

            // We now have all the indices we need to reference the neighbors of
            // the current cell, and we can construct the current state by using
            // the upper/lower and left/right indices to reference neighboring
            // cells
            let state = [
                self.rows[upper_neighbor_y_index][left_neighbor_x_index], // Upper Left
                self.rows[upper_neighbor_y_index][col],                   // Upper middle
                self.rows[upper_neighbor_y_index][right_neighbor_x_index], // upper right
                self.rows[row][left_neighbor_x_index],                    // Middle left
                self.rows[row][col],                                      // Current cell
                self.rows[row][right_neighbor_x_index],                   // Middle right
                self.rows[lower_neighbor_y_index][left_neighbor_x_index], // Lower left
                self.rows[lower_neighbor_y_index][col],                   // Lower middle
                self.rows[lower_neighbor_y_index][right_neighbor_x_index], // Lower right
            ];

            let next_cell_state = match self.rules.get(&state.into()) {
                Some(next_cell_state) => *next_cell_state,
                None => CellState::Off,
            };

            next_state[row][col] = next_cell_state;
        }
        self.rows = next_state;
    }

    fn draw(&self, frame: &mut [u8]) {
        // Iterate over the 4 bytes making up the Red-Green-Blue-Alpha (RGBA)
        // pixel colors
        for (i, rgba_pixel) in frame.chunks_exact_mut(4).enumerate() {
            let row = i / WIDTH_USIZE;
            let col = i % WIDTH_USIZE;

            let rgba = match self.rows[row][col] {
                CellState::On => [0xff, 0xff, 0xff, 0xff],
                CellState::Off => [0x59, 0x57, 0x52, 0xff],
                // CellState::On => [0xf3, 0x7c, 0x1f, 0xff],
                // CellState::Off => [0x59, 0x57, 0x52, 0xff],
            };
            // let rgba = [
            //     rgba[0] / std::cmp::max(rgba_pixel[0], 1),
            //     rgba[1] / std::cmp::max(rgba_pixel[1], 1),
            //     rgba[2] / std::cmp::max(rgba_pixel[2], 1),
            //     rgba[3] / std::cmp::max(rgba_pixel[3], 1),
            // ];
            rgba_pixel.copy_from_slice(&rgba);
        }
    }
}

impl From<u32> for RuleState {
    fn from(value: u32) -> Self {
        RuleState(value)
    }
}

impl From<[CellState; 9]> for InputState {
    fn from(value: [CellState; 9]) -> Self {
        Self([
            value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
            value[8],
        ])
    }
}

impl From<u32> for InputState {
    fn from(value: u32) -> Self {
        Self([
            if ((value >> 0) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 1) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 2) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 3) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 4) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 5) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 6) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 7) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
            if ((value >> 8) & 0x1 == 1) {
                CellState::On
            } else {
                CellState::Off
            },
        ])
    }
}
impl From<[CellState; 9]> for RuleState {
    fn from(pixel_states: [CellState; 9]) -> Self {
        let pixel_states_as_u32 = pixel_states
            .iter()
            .map(|pixel_state| pixel_state.into())
            .collect::<Vec<u32>>();

        let output_state = pixel_states_as_u32[0] << 0
            | pixel_states_as_u32[1] << 1
            | pixel_states_as_u32[2] << 2
            | pixel_states_as_u32[3] << 3
            | pixel_states_as_u32[4] << 4
            | pixel_states_as_u32[5] << 5
            | pixel_states_as_u32[6] << 6
            | pixel_states_as_u32[7] << 7
            | pixel_states_as_u32[8] << 8;

        RuleState(output_state)
    }
}

impl From<&CellState> for u32 {
    fn from(value: &CellState) -> Self {
        match value {
            CellState::On => 1,
            CellState::Off => 0,
        }
    }
}
impl From<CellState> for bool {
    fn from(value: CellState) -> Self {
        match value {
            CellState::On => true,
            CellState::Off => false,
        }
    }
}

// impl WorldxK {
//     /// Create a new `World` instance that can draw a moving box.
//     fn new() -> Self {
//         Self {
//             box_x: 24,
//             box_y: 16,
//             velocity_x: 1,
//             velocity_y: 1,
//         }
//     }

//     /// Update the `World` internal state; bounce the box around the screen.
//     fn update(&mut self) {
//         if self.box_x <= 0 || self.box_x + BOX_SIZE > WIDTH as i16 {
//             self.velocity_x *= -1;
//         }
//         if self.box_y <= 0 || self.box_y + BOX_SIZE > HEIGHT as i16 {
//             self.velocity_y *= -1;
//         }

//         self.box_x += self.velocity_x;
//         self.box_y += self.velocity_y;
//     }

//     /// Draw the `World` state to the frame buffer.
//     ///
//     /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
//     fn draw(&self, frame: &mut [u8]) {
//         for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
//             let x = (i % WIDTH as usize) as i16;
//             let y = (i / WIDTH as usize) as i16;

//             let inside_the_box = x >= self.box_x
//                 && x < self.box_x + BOX_SIZE
//                 && y >= self.box_y
//                 && y < self.box_y + BOX_SIZE;

//             let rgba = if inside_the_box {
//                 [0x5e, 0x48, 0xe8, 0xff]
//             } else {
//                 [0x48, 0xb2, 0xe8, 0xff]
//             };

//             pixel.copy_from_slice(&rgba);
//         }
//     }
// }
