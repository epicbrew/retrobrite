use sdl2::{video::Window, EventPump, Sdl, VideoSubsystem};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;

pub struct Gui {
    sdl_context: Sdl,
    video_subsystem: VideoSubsystem,
    canvas: Canvas<Window>,
    event_pump: EventPump,
    frame_buffer: [u8; 256*240*3],
}

impl Gui {
    pub fn init() -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem.window("Retrobrite", 256*3, 240*3)
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
                frame_buffer: [0; 256*240*3],
            }
        )
    }

    pub fn set_pixel(&mut self, x: u16, y: u16, value: u8) {
        let xu = x as usize;
        let yu = y as usize;
        let index = ((yu * 256*3) + xu*3) as usize;

        // RGBX8888 pixel format
        let color_value: u8 = match value {
            0 => 0x0,
            1 => 0x80,
            2 => 0xC8,
            3 => 0xFF,
            _ => panic!("Invalid color value (for now)"),
                
        };

        self.frame_buffer[index] = color_value;
        self.frame_buffer[index+1] = color_value;
        self.frame_buffer[index+2] = color_value;
    }

    pub fn render_frame(&mut self) {
        let texture_creator = self.canvas.texture_creator();
        let surface = Surface::from_data(&mut self.frame_buffer,
                                         256, 240, 256*3, 
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