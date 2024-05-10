use sdl2::{video::Window, EventPump, Sdl, VideoSubsystem};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;

/// NES resolution width
const WIDTH: u32 = 256;

// NES resolution height minus 16 scanlines of overscan
const HEIGHT: u32 = 224;

// This omits the first 8 scanlines of the display. This value
// cannot be set above 16 without changing HEIGHT. A value of
// 8 will have the effect of omitting the top 8 and bottom 8
// scanlines. A value of 16 would omit the top 16 scanlines and
// 0 bottom scanlines.
const TOP_OVERSCAN: u16 = 8;

const FRAME_BUFFER_SIZE_IN_BYTES: usize = (WIDTH * HEIGHT * 3) as usize;

const PALETTE: [[u8; 3]; 64] = [
        [0x62, 0x62, 0x62],
        [0x00, 0x1f, 0xb2],
        [0x24, 0x04, 0xc8],
        [0x52, 0x00, 0xb2],
        [0x73, 0x00, 0x76],
        [0x80, 0x00, 0x24],
        [0x73, 0x0b, 0x00],
        [0x52, 0x28, 0x00],
        [0x24, 0x44, 0x00],
        [0x00, 0x57, 0x00],
        [0x00, 0x5c, 0x00],
        [0x00, 0x53, 0x24],
        [0x00, 0x3c, 0x76],
        [0x00, 0x00, 0x00],
        [0x00, 0x00, 0x00],
        [0x00, 0x00, 0x00],

        [0xab, 0xab, 0xab],
        [0x0d, 0x57, 0xff],
        [0x4b, 0x30, 0xff],
        [0x8a, 0x13, 0xff],
        [0xbc, 0x08, 0xd6],
        [0xd2, 0x12, 0x69],
        [0xc7, 0x2e, 0x00],
        [0x9d, 0x54, 0x00],
        [0x60, 0x7b, 0x00],
        [0x20, 0x98, 0x00],
        [0x00, 0xa3, 0x00],
        [0x00, 0x99, 0x42],
        [0x00, 0x7d, 0xb4],
        [0x00, 0x00, 0x00],
        [0x00, 0x00, 0x00],
        [0x00, 0x00, 0x00],

        [0xff, 0xff, 0xff],
        [0x53, 0xae, 0xff],
        [0x90, 0x85, 0xff],
        [0xd3, 0x65, 0xff],
        [0xff, 0x57, 0xff],
        [0xff, 0x5d, 0xcf],
        [0xff, 0x77, 0x57],
        [0xfa, 0x9e, 0x00],
        [0xbd, 0xc7, 0x00],
        [0x7a, 0xe7, 0x00],
        [0x43, 0xf6, 0x11],
        [0x26, 0xef, 0x7e],
        [0x2c, 0xd5, 0xf6],
        [0x4e, 0x4e, 0x4e],
        [0x00, 0x00, 0x00],
        [0x00, 0x00, 0x00],

        [0xff, 0xff, 0xff],
        [0xb6, 0xe1, 0xff],
        [0xce, 0xd1, 0xff],
        [0xe9, 0xc3, 0xff],
        [0xff, 0xbc, 0xff],
        [0xff, 0xbd, 0xf4],
        [0xff, 0xc6, 0xc3],
        [0xff, 0xd5, 0x9a],
        [0xe9, 0xe6, 0x81],
        [0xce, 0xf4, 0x81],
        [0xb6, 0xfb, 0x9a],
        [0xa9, 0xfa, 0xc3],
        [0xa9, 0xf0, 0xf4],
        [0xb8, 0xb8, 0xb8],
        [0x00, 0x00, 0x00],
        [0x00, 0x00, 0x00],
];

pub struct Gui {
    // I think we need to keep sdl_context and video subsystem around
    // so allowing dead_code here.
    #[allow(dead_code)]
    sdl_context: Sdl,
    #[allow(dead_code)]
    video_subsystem: VideoSubsystem,
    canvas: Canvas<Window>,
    event_pump: EventPump,
    frame_buffer: [u8; FRAME_BUFFER_SIZE_IN_BYTES],
}

impl Gui {
    pub fn init() -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem.window("Retrobrite", WIDTH*3, HEIGHT*3)
            .position_centered()
            .build()
            .expect("Could not create window");

        let canvas = window.into_canvas().build().expect("could not creat canvas");


        let event_pump = sdl_context.event_pump()?;

        Ok(
            Self {
                sdl_context,
                video_subsystem,
                canvas,
                event_pump,
                frame_buffer: [0; FRAME_BUFFER_SIZE_IN_BYTES],
            }
        )
    }

    pub fn set_pixel(&mut self, x: u16, y: u16, value: u8) {
        // Skip scanlines in the overscan portion of the top of the screen
        if y < TOP_OVERSCAN {
            return;
        }

        // Skip overscan lines at the bottom of the screen that would be out
        // of range of our frame buffer.
        if y - TOP_OVERSCAN >= HEIGHT as u16 {
            return;
        }

        let xu = x as usize;
        let yu = (y - TOP_OVERSCAN) as usize;
        let index = (yu * WIDTH as usize * 3) + xu*3;

        // Mod value by 64 because some games rely on the palette value wrapping
        // after 64
        let rgb = PALETTE[(value % 64) as usize];
        self.frame_buffer[index]   = rgb[0];
        self.frame_buffer[index+1] = rgb[1];
        self.frame_buffer[index+2] = rgb[2];
    }

    pub fn render_frame(&mut self) {
        let texture_creator = self.canvas.texture_creator();
        let surface = Surface::from_data(&mut self.frame_buffer,
                                         WIDTH, HEIGHT, WIDTH*3, 
                                         PixelFormatEnum::RGB24).unwrap();

        let texture = surface.as_texture(&texture_creator).unwrap();

        self.canvas.copy(&texture, None, None).expect("failed to copy texture");
        self.canvas.present();
    }

    pub fn process_events(&mut self) -> bool {
        let mut result = true;
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    result = false;
                },
                _ => {}
            }
        }

        result
    }
}