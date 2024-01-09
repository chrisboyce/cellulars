#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::collections::HashMap;

use error_iter::ErrorIter as _;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

// Convenience variable so we don't have to use `as usize` to "cast" the `u32`
// (4 bytes) to a `usize` (4 bytes on a 64-bit system, but the actual length
// it varies depending on the architecture you're compiling for) whenever we
// want to use the height or width with array indices (which are usually
// represented with `usize`)
const WIDTH_USIZE: usize = WIDTH as usize;
const HEIGHT_USIZE: usize = HEIGHT as usize;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum CellState {
    On,
    Off,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct RuleState(u32);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct InputState([CellState; 9]);

// Unused so far..
struct Rule {
    input: [CellState; 3],
    output: CellState,
}

type Rules = HashMap<InputState, CellState>;

struct World {
    /// Each row contains `WIDTH` number of pixels, and each pixel is
    /// represented by their `State`
    rows: [[CellState; WIDTH_USIZE]; HEIGHT_USIZE],
    rules: Rules,
    generation: u32,
}

fn main() -> Result<(), Error> {
    // let mut rules = [PixelState::Off; 362880];
    // for rule in rules.iter_mut() {
    //     *rule = if rand::random() {
    //         PixelState::On
    //     } else {
    //         PixelState::Off
    //     };
    // }
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH, HEIGHT);
        WindowBuilder::new()
            .with_title("celluars")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };
    let mut world = World::new();

    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.frame_mut());
            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::U) {
                world.rules = randomize_rules();
            }
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

            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            // Update internal state and request a redraw
            world.update();
            window.request_redraw();
        }
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

fn randomize_rules() -> Rules {
    let mut rules = Rules::with_capacity(362880);
    for i in 0..362880_u32 {
        let rule = if rand::random::<f32>() < 0.1 {
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
        let rules = randomize_rules();

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

            // let neighbors = [
            //     state[0], state[1], state[2], state[3], state[5], state[6], state[7], state[8],
            // ];
            // // Given the current state, determine the next state for the current
            // // cell. We use the rules for Conways Game of Life to determine
            // // the next state, which depends on the number of neighboring cells.

            // let current_state = self.rows[row][col];
            // let living_neighbor_count = neighbors
            //     .into_iter()
            //     .filter(|cell| matches!(cell, PixelState::On))
            //     .count();

            // // The rules for Conway's game of life are:
            // // 1) Any live cell with fewer than two live neighbours dies, as if by underpopulation.
            // // 2) Any live cell with two or three live neighbours lives on to the next generation.
            // // 3) Any live cell with more than three live neighbours dies, as if by overpopulation.
            // // 4) Any dead cell with exactly three live neighbours becomes a live cell, as if by reproduction.

            // let next_cell_state = if matches!(current_state, PixelState::On) {
            //     if living_neighbor_count < 2 {
            //         PixelState::Off
            //     } else if (living_neighbor_count == 2 || living_neighbor_count == 3) {
            //         PixelState::On
            //     } else if living_neighbor_count > 3 {
            //         PixelState::Off
            //     } else {
            //         current_state
            //     }
            // } else {
            //     if living_neighbor_count == 3 {
            //         PixelState::On
            //     } else {
            //         current_state
            //     }
            // };

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
            rgba_pixel.copy_from_slice(&rgba);
        }
    }
}

// All we really want to do is to be able to convert a 16-bit number into our
// rule state.
// struct R{
//     input: [PixelState;9],
//     output: PixelState
// };

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
