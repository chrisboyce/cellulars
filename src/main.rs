#![deny(clippy::all)]
#![forbid(unsafe_code)]

use error_iter::ErrorIter as _;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

// Convenience variable so we don't have to use `as usize` to "cast" the `u32`
// (4 bytes) to a `usize` (4 bytes on a 64-bit system, but the actual length
// it varies depending on the architecture you're compiling for) whenever we
// want to use the height or width with array indices (which are usually
// represented with `usize`)
const WIDTH_USIZE: usize = WIDTH as usize;
const HEIGHT_USIZE: usize = HEIGHT as usize;

#[derive(Copy, Clone)]
enum PixelState {
    On,
    Off,
}

// Unused so far..
struct Rule {
    input: [PixelState; 3],
    output: PixelState,
}

struct World {
    /// Each row contains `WIDTH` number of pixels, and each pixel is
    /// represented by their `State`
    rows: [[PixelState; WIDTH_USIZE]; HEIGHT_USIZE],
    generation: u64,
}

fn main() -> Result<(), Error> {
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
            if input.key_pressed(VirtualKeyCode::R) {
                for i in 0..HEIGHT_USIZE {
                    for j in 0..WIDTH_USIZE {
                        world.rows[i][j] = if rand::random() {
                            PixelState::On
                        } else {
                            PixelState::Off
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

impl World {
    fn new() -> Self {
        let mut default = Self {
            rows: [[PixelState::Off; WIDTH_USIZE]; HEIGHT_USIZE],
            generation: 0,
        };
        for i in 0..WIDTH_USIZE {
            default.rows[0][i] = if rand::random() {
                PixelState::On
            } else {
                PixelState::Off
            };
        }

        default
    }

    fn update(&mut self) {
        let mut next_state = [[PixelState::Off; WIDTH_USIZE]; HEIGHT_USIZE];
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
            let state = (
                self.rows[upper_neighbor_y_index][left_neighbor_x_index], // Upper Left
                self.rows[upper_neighbor_y_index][col],                   // Upper middle
                self.rows[upper_neighbor_y_index][right_neighbor_x_index], // upper right
                self.rows[row][left_neighbor_x_index],                    // Middle left
                self.rows[row][col],                                      // Current cell
                self.rows[row][right_neighbor_x_index],                   // Middle right
                self.rows[lower_neighbor_y_index][left_neighbor_x_index], // Lower left
                self.rows[lower_neighbor_y_index][col],                   // Lower middle
                self.rows[lower_neighbor_y_index][right_neighbor_x_index], // Lower right
            );

            let neighbors = [
                state.0, state.1, state.2, state.3, state.5, state.6, state.7, state.8,
            ];
            // Given the current state, determine the next state for the current
            // cell. We use the rules for Conways Game of Life to determine
            // the next state, which depends on the number of neighboring cells.

            let current_state = self.rows[row][col];
            let living_neighbor_count = neighbors
                .into_iter()
                .filter(|cell| matches!(cell, PixelState::On))
                .count();

            //  The rules for Conway's game of life are:
            // 1)  Any live cell with fewer than two live neighbours dies, as if by underpopulation.
            // 2) Any live cell with two or three live neighbours lives on to the next generation.
            // 3) Any live cell with more than three live neighbours dies, as if by overpopulation.
            // 4) Any dead cell with exactly three live neighbours becomes a live cell, as if by reproduction.

            let next_cell_state = if matches!(current_state, PixelState::On) {
                if living_neighbor_count < 2 {
                //if living_neighbor_count < 1 {   
                    PixelState::Off
                } else if (living_neighbor_count == 2 || living_neighbor_count == 3) {
                    PixelState::On
                } else if living_neighbor_count > 2 {    
                //} else if living_neighbor_count > 3 {
                    PixelState::Off
                } else {
                    current_state
                }
            } else {
                if living_neighbor_count == 3 {
                    PixelState::On
                } else {
                    current_state
                }
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
                //PixelState::On => [0xf3, 0x7c, 0x1f, 0xff], //Reddish brown w/full opacity
                //PixelState::On => [0x00, 0xAF, 0xFF, 0xff], //light blue w/full opacity
                PixelState::Off => [0x00, 0x00, 0x00, 0xff], //Black
                //PixelState::Off => [0x59, 0x57, 0x52, 0xff],
                PixelState::On => [0xFF, 0xFF, 0xFF, 0xFF], //White
            };
            rgba_pixel.copy_from_slice(&rgba);
        }
    }
}
