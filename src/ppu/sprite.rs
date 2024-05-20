use crate::state::NesState;
use crate::utils;

#[derive(Debug)]
#[derive(Default)]
pub enum SpriteSize {
    #[default] Sprite8x8,
    Sprite8x16,
}

/// Sprite priority from OAM byte 2. Either render in front of
/// or behind background.
#[derive(Default)]
pub enum SpriteBgPriority {
    #[default] InFrontOfBackground,
    BehindBackground,
}

pub struct SpritePixel {
    pub(super) palette_value: u8,
    pub(super) color_index: u8,
    pub(super) priority: SpriteBgPriority,
    pub(super) sprite_0: bool,
}


///
/// Buffer for sprite data needed for rendering next scanline.
/// These buffers are initialized from secondary OAM during HBlank.
///
#[derive(Default)]
pub struct SpriteBuffer {
    /// X position of left side of sprite.
    x_position: u8,

    /// Y position of top of sprite.
    y_position: u8,

    /// Sprite palette to use in rendering.
    palette: u8,

    /// Priority relative to the background.
    priority: SpriteBgPriority,

    /// True if this buffer contains "sprite 0".
    is_sprite_0: bool,

    /// If true, render sprite flipped horizontally.
    flip_horizontally: bool,

    /// If true, render sprite flipped vertically.
    flip_vertically: bool,

    /// Least significant bit plane of pattern table tile.
    pattern_tile_lsb: u8,

    /// Most significant bit plane of pattern table tile.
    pattern_tile_msb: u8,
}

impl SpriteBuffer {
    /// Load data from OAM into the sprite buffer.
    pub(super) fn load(&mut self, 
                       y: u8,
                       tile: u8,
                       attributes: u8,
                       x: u8,
                       is_sprite_0: bool,
                       pattern_table_addr_8x8: u16,
                       sprite_size: SpriteSize,
                       scanline: u16,
                       state: &mut NesState)
    {
        self.x_position = x;
        self.y_position = y;

        self.palette = attributes & 0x03;
        self.priority = if utils::bit_is_set(5, attributes) {
            SpriteBgPriority::BehindBackground
        } else {
            SpriteBgPriority::InFrontOfBackground
        };

        self.is_sprite_0 = is_sprite_0;

        self.flip_horizontally = utils::bit_is_set(6, attributes);
        self.flip_vertically = utils::bit_is_set(7, attributes);

        self.set_pattern_lsb_msb(sprite_size, scanline, y, pattern_table_addr_8x8, tile, state);
    }

    /// Gets the value to select from the sprite's palette (i.e. 0, 1, 2, or 3).
    pub(super) fn get_palette_value(&self, screen_x: u8) -> u8 {
        let bit = if self.flip_horizontally {
            screen_x - self.x_position
        } else {
            7 - screen_x - self.x_position
        };

        let lsb = if utils::bit_is_set(bit, self.pattern_tile_lsb) { 1 } else { 0 };
        let msb = if utils::bit_is_set(bit, self.pattern_tile_msb) { 1 } else { 0 };

        msb << 1 | lsb
    }

    fn set_pattern_lsb_msb(&mut self,
                           sprite_size: SpriteSize,
                           scanline: u16,
                           y: u8,
                           pattern_table_addr_8x8: u16,
                           tile: u8,
                           state: &mut NesState)
    {
        match sprite_size {
            SpriteSize::Sprite8x8 => {
                let intra_tile_y = if self.flip_vertically {
                    7 - (scanline - (y as u16))
                } else {
                    scanline - (y as u16)
                };
    
                let tile_addr = pattern_table_addr_8x8 | ((tile as u16) << 4) | intra_tile_y;
    
                self.pattern_tile_lsb = state.ppu_mem_read(tile_addr);
    
                // Or tile_addr with 0x8 to set bit 3 for msb bitplane
                self.pattern_tile_msb = state.ppu_mem_read(tile_addr | 0x8);
    
            },
            SpriteSize::Sprite8x16 => {
                todo!();
            }
        };
    }

    pub fn x_position(&self) -> u8 {
        self.x_position
    }

    pub fn palette(&self) -> u8 {
        self.palette
    }

    pub fn priority(&self) -> SpriteBgPriority {
        match self.priority {
            SpriteBgPriority::InFrontOfBackground => SpriteBgPriority::InFrontOfBackground,
            SpriteBgPriority::BehindBackground => SpriteBgPriority::BehindBackground,
        }
    }

    pub fn is_sprite_0(&self) -> bool {
        self.is_sprite_0
    }
}


///
/// Holds state needed during sprite evaluation on each visible scanline.
/// 
pub struct PpuSpriteEvalState {
    /// During sprite evaluation, keeps track of how many sprites have been
    /// found to be on the scanline being evaluated. After eight sprites
    /// have been found, the spite overflow flag logic must be invoked.
    //num_sprites_on_scanline: u8,

    /// During sprite evaluation this is the current OAM address we
    /// are reading from.
    pub(super) oam_addr: u16,

    /// Secondary Object Attribute Memory (OAM)
    /// Holds up to 8 OAM entries.
    pub(super) secondary_oam: [[u8; 4]; 8],

    /// Index to use for next secondary oam write.
    pub(super) secondary_oam_index: usize,

    /// If true, then the first entry in secondary OAM is sprite 0.
    /// When this flag is true, we need to check for sprite 0 hits.
    pub(super) maybe_sprite_0_hit: bool,

    /// If true then sprite overflow has occurred on this scanline.
    pub(super) sprite_overflow: bool,

    pub(super) sprite_buffers: Vec<SpriteBuffer>,
}

impl Default for PpuSpriteEvalState {
    fn default() -> Self {
        let mut sprite_buffers = Vec::new();

        for _ in 0..8 {
            sprite_buffers.push(SpriteBuffer::default());
        }

        Self { 
            //num_sprites_on_scanline: 0,
            oam_addr: 0,
            secondary_oam: [[0xFF; 4]; 8], // Initialize in "cleared" (0xFF) state
            secondary_oam_index: 0,
            maybe_sprite_0_hit: false,
            sprite_overflow: false,
            sprite_buffers,
        }
    }
}

impl PpuSpriteEvalState {

    pub fn reset(&mut self) {
        //self.num_sprites_on_scanline = 0;
        self.oam_addr = 0;
        self.secondary_oam_index = 0;
        self.maybe_sprite_0_hit = false;
        self.sprite_overflow = false;
    }
}