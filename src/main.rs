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
    Betwixt,
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
        };
        default.rows[0][WIDTH_USIZE / 2] = PixelState::On;

        default
    }

    fn update(&mut self) {
        for i in 0..(WIDTH_USIZE * HEIGHT_USIZE) {
            let row = i / WIDTH_USIZE;
            let col = i % WIDTH_USIZE;

            // If the current column is 0, then look to the right-most column,
            // to treat the row as if it wraps around
            let left_neighbor_state = if col == 0 {
                self.rows[row][WIDTH_USIZE - 1]
            } else {
                self.rows[row][col - 1]
            };

            // If the current column is at the end of the row, look to the
            // left-most colum to treat the row as if it wraps around
            let right_neighbor_state = if col + 1 >= WIDTH_USIZE {
                self.rows[row][0]
            } else {
                self.rows[row][col + 1]
            };

            let input_state = [
                left_neighbor_state,
                self.rows[row][col],
                right_neighbor_state,
            ];

            // This defines the rules for cell progression. Given the left
            // neighbor, current value, and right neighbor, the output state
            // can be varied.
            let output_state = match input_state {
                [PixelState::On, PixelState::On, PixelState::On] => PixelState::Betwixt,
                [PixelState::On, PixelState::On, PixelState::Off] => PixelState::Off,
                [PixelState::On, PixelState::Off, PixelState::On] => PixelState::On,
                [PixelState::On, PixelState::Off, PixelState::Off] => PixelState::On,
                [PixelState::Off, PixelState::On, PixelState::On] => PixelState::On,
                [PixelState::Off, PixelState::On, PixelState::Off] => PixelState::On,
                [PixelState::Off, PixelState::Off, PixelState::On] => PixelState::On,
                [PixelState::Off, PixelState::Off, PixelState::Off] => PixelState::Off,
                [PixelState::On, PixelState::On, PixelState::Betwixt] => PixelState::Off,
                [PixelState::On, PixelState::Betwixt, PixelState::On] => PixelState::On,
                [PixelState::On, PixelState::Betwixt, PixelState::Betwixt] => PixelState::Betwixt,
                [PixelState::On, PixelState::Betwixt, PixelState::Off] => PixelState::Off,
                [PixelState::On, PixelState::Off, PixelState::Betwixt] => PixelState::On,
                [PixelState::Betwixt, PixelState::On, PixelState::On] => PixelState::Betwixt,
                [PixelState::Betwixt, PixelState::On, PixelState::Betwixt] => PixelState::Off,
                [PixelState::Betwixt, PixelState::On, PixelState::Off] => PixelState::On,
                [PixelState::Betwixt, PixelState::Betwixt, PixelState::On] => PixelState::Betwixt,
                [PixelState::Betwixt, PixelState::Betwixt, PixelState::Betwixt] => PixelState::Off,
                [PixelState::Betwixt, PixelState::Betwixt, PixelState::Off] => PixelState::On,
                [PixelState::Betwixt, PixelState::Off, PixelState::On] => PixelState::Betwixt,
                [PixelState::Betwixt, PixelState::Off, PixelState::Betwixt] => PixelState::Off,
                [PixelState::Betwixt, PixelState::Off, PixelState::Off] => PixelState::On,
                [PixelState::Off, PixelState::On, PixelState::Betwixt] => PixelState::Betwixt,
                [PixelState::Off, PixelState::Betwixt, PixelState::On] => PixelState::Off,
                [PixelState::Off, PixelState::Betwixt, PixelState::Betwixt] => PixelState::On,
                [PixelState::Off, PixelState::Betwixt, PixelState::Off] => PixelState::Off,
                [PixelState::Off, PixelState::Off, PixelState::Betwixt] => PixelState::Off,
            };

            // Set the value for the next row
            if row < HEIGHT_USIZE - 1 {
                self.rows[row + 1][col] = output_state;
            }
        }
    }

    fn draw(&self, frame: &mut [u8]) {
        // Iterate over the 4 bytes making up the Red-Green-Blue-Alpha (RGBA)
        // pixel colors
        for (i, rgba_pixel) in frame.chunks_exact_mut(4).enumerate() {
            let row = i / WIDTH_USIZE;
            let col = i % WIDTH_USIZE;

            let rgba = match self.rows[row][col] {
                PixelState::On => [0xa8, 0x8c, 0x7d, 0xff],
                PixelState::Off => [0x54, 0x73, 0x8e, 0xff],
                PixelState::Betwixt => [0x9d, 0xba, 0x94, 0xff],
            };
            rgba_pixel.copy_from_slice(&rgba);
        }
    }
}
