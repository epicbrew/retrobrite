use crate::utils::{self, bit_is_set, clear_bit, set_bit, set_bits_from_mask_u16};
use crate::mem::Memory;

/// Size of Object Attribute Memory.
const OAM_SIZE: usize = 256;

#[derive(Debug)]
pub enum PpuCycleResult {
    Idle,
    Pixel {scanline: u16, x: u16, color: u8},
    HBlank {scanline: u16, cycle: u16},
    PostRenderLine,
    VBlankLine {trigger_nmi: bool, scanline: u16},
    PreRenderLine,
}

// Latch for ppuaddr, ppuscroll, etc.
//pub enum Latch {
//    /// Latch state clear.
//    Clear,
//
//    /// Latch state set with value.
//    Set(u8),
//}

/// Toggle to represent internal PPU register 'w'.
#[derive(Default)]
enum Toggle {
    /// w = 0
    #[default] FirstWrite,
    /// w = 1
    SecondWrite,
}

impl Toggle {
    fn toggle(&mut self) {
        match *self {
            Toggle::FirstWrite => *self = Toggle::SecondWrite,
            Toggle::SecondWrite => *self = Toggle::FirstWrite,
        }
    }
    
    fn _print(&self) {
        match *self {
            Toggle::FirstWrite => println!("first write"),
            Toggle::SecondWrite => println!("second write"),
        }
    }
}

#[derive(Default)]
enum SpriteSize {
    #[default] Sprite8x8,
    Sprite8x16,
}

///
/// PPUCTRL fields. This struct receives the raw value written to PPUCTRL
/// and parses the value into fields.
///
#[derive(Default)]
struct PpuCtrl {
    /// Raw value written via $2000
    value: u8,
    base_nt_addr: u16,
    vram_increment: u16,
    sprite_pt_addr_8x8: u16,
    bg_pt_addr: u16,
    sprite_size: SpriteSize,
    generate_nmi: bool
}

impl PpuCtrl {
    fn update(&mut self, value: u8) {
        self.value = value;
        self.base_nt_addr = self.get_base_nt_addr();
        self.vram_increment = self.get_vram_increment();
        self.sprite_pt_addr_8x8 = self.get_sprite_pt_addr();
        self.bg_pt_addr = self.get_bg_pt_addr();
        self.sprite_size = self.get_sprite_size();
        self.generate_nmi = self.get_nmi_generate();
    }

    fn get_base_nt_addr(&self) -> u16 {
        let index = self.value & 0x3;
        match index {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!("ppu_ctrl_get_base_nametable_addr: invalid index")
        }
    }

    fn get_vram_increment(&self) -> u16 {
        if utils::bit_is_set(2, self.value) {
            32
        } else {
            1
        }
    }

    fn get_sprite_pt_addr(&self) -> u16 {
        if utils::bit_is_set(3, self.value) {
            0x1000
        } else {
            0x0
        }
    }

    fn get_bg_pt_addr(&self) -> u16 {
        if utils::bit_is_set(4, self.value) {
            0x1000
        } else {
            0x0
        }
    }

    fn get_sprite_size(&self) -> SpriteSize {
        if utils::bit_is_set(5, self.value) {
            SpriteSize::Sprite8x16
        } else {
            SpriteSize::Sprite8x8
        }
    }

    fn get_nmi_generate(&self) -> bool {
        if utils::bit_is_set(7, self.value) {
            true
        } else {
            false
        }
    }
}

///
/// PPUMASK fields. This struct receives the raw value written to PPUMASK
/// and parses the value into fields.
/// 7  bit  0
/// ---- ----
/// BGRs bMmG
/// |||| ||||
/// |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
/// |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
/// |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
/// |||| +---- 1: Show background
/// |||+------ 1: Show sprites
/// ||+------- Emphasize red (green on PAL/Dendy)
/// |+-------- Emphasize green (red on PAL/Dendy)
/// +--------- Emphasize blue
///
#[derive(Default)]
struct PpuMask {
    /// Raw value written via $2001
    value: u8,
    greyscale: bool,
    show_bg_leftmost_8px: bool,
    show_sprites_leftmost_8px: bool,
    render_bg: bool,
    render_sprites: bool,
    emphasize_red: bool,
    emphasize_green: bool,
    emphasize_blue: bool,
}

impl PpuMask {
    fn update(&mut self, value: u8) {
        self.value = value;
        self.greyscale = bit_is_set(0, value);
        self.show_bg_leftmost_8px = bit_is_set(1, value);
        self.show_sprites_leftmost_8px = bit_is_set(2, value);
        self.render_bg = bit_is_set(3, value);
        self.render_sprites = bit_is_set(4, value);
        self.emphasize_red = bit_is_set(5, value);
        self.emphasize_green = bit_is_set(6, value);
        self.emphasize_blue = bit_is_set(7, value);
    }
}

///
/// Ppu registers.
/// Descriptions per https://www.nesdev.org/wiki/PPU_registers.
///
#[derive(Default)]
struct PpuRegisters {
    ///
    /// PPU control (Cpu: $2000, Bits: VPHB SINN).
    /// NMI enable (V), PPU master/slave (P), sprite height (H), background tile select (B),
    /// sprite tile select (S), increment mode (I), nametable select (NN)
    /// 
    ppu_ctrl: PpuCtrl,

    ///
    /// PPU Mask (Cpu: $2001, Bits: BGRs bMmG).
    /// color emphasis (BGR), sprite enable (s), background enable (b),
    /// sprite left column enable (M), background left column enable (m), greyscale (G)
    /// 
    ppu_mask: PpuMask,

    ///
    /// PPU Status (Cpu: $2002, Bits: VSO- ----).
    /// vblank (V), sprite 0 hit (S), sprite overflow (O).
    /// Read resets write pair for $2005/$2006.
    /// 
    ppu_status: u8,

    ///
    /// OAM Address (Cpu: $2003, Bits: aaaa aaaa).
    /// OAM read/write address.
    /// 
    oam_addr: u8,

    ///
    /// PPU internal register for current VRAM address (15 bits)
    /// 
    v: u16,

    ///
    /// PPU internal register for temporary VRAM address (15 bits); can also be
    /// thought of as the address of the top left onscreen tile.
    /// 
    t: u16,

    ///
    /// Fine X scroll (3 bits)
    /// 
    x: u8,

    ///
    /// Write toggle flag (1 bit)
    /// 
    w: Toggle,
}

#[derive(Default)]
#[derive(Debug)]
enum PpuBgFetchState {
    #[default] Idle,
    NametableAddr,
    NametableRead,
    AttrtableAddr,
    AttrtableRead,
    BackgroundLSBAddr,
    BackgroundLSBRead,
    BackgroundMSBAddr,
    BackgroundMSBRead,
}

impl PpuBgFetchState {
    fn next(&mut self) {
        match *self {
            PpuBgFetchState::Idle              => *self = PpuBgFetchState::Idle,
            PpuBgFetchState::NametableAddr     => *self = PpuBgFetchState::NametableRead,
            PpuBgFetchState::NametableRead     => *self = PpuBgFetchState::AttrtableAddr,
            PpuBgFetchState::AttrtableAddr     => *self = PpuBgFetchState::AttrtableRead,
            PpuBgFetchState::AttrtableRead     => *self = PpuBgFetchState::BackgroundLSBAddr,
            PpuBgFetchState::BackgroundLSBAddr => *self = PpuBgFetchState::BackgroundLSBRead,
            PpuBgFetchState::BackgroundLSBRead => *self = PpuBgFetchState::BackgroundMSBAddr,
            PpuBgFetchState::BackgroundMSBAddr => *self = PpuBgFetchState::BackgroundMSBRead,
            PpuBgFetchState::BackgroundMSBRead => *self = PpuBgFetchState::NametableAddr
        }
    }
}

#[derive(Default)]
struct ShiftRegister16Bit {
    /// Lower 8 bits of register.
    lo: u8,
    /// Upper 8 bits of register.
    hi: u8,
}

/// Shift register used for holding rendering data/state.
impl ShiftRegister16Bit {
    fn push_byte(&mut self, byte: u8) {
        self.lo = self.hi;  // Shift upper byte to lower byte
        self.hi = byte;     // Set upper byte to new byte
    }
}

#[derive(Default)]
struct PpuBgRenderState {
    fetch_state: PpuBgFetchState,
    /// Address of nametable tile to be fetched.
    tile_addr: u16,
    /// Value read from tile_addr.
    tile_value: u8,

    /// Address of attribute table data for tile.
    attribute_addr: u16,

    /// Attribute data read from attribute_addr.
    attribute_data: u8,

    /// Address for tile pattern table data lsb.
    bg_lsb_addr: u16,

    /// Tile's pattern table data lsb.
    bg_lsb: u8,

    /// Address for tile pattern table data msb.
    bg_msb_addr: u16,

    /// Tile's pattern table data msb.
    bg_msb: u8,

    pattern_tile_msb_register: ShiftRegister16Bit,
    pattern_tile_lsb_register: ShiftRegister16Bit,

    /// Shift register for attribute/palette most significant bit.
    /// Lower 8 bits of this are selected every hori(v) and set to
    /// either 0xFF or 0x00 to emulate the PPU 1-bit latch in a way
    /// that is easy to shift each pixel.
    attribute_msb_shift_register: u16,

    /// Shift register for attribute/palette least significant bit.
    /// Lower 8 bits of this are selected every hori(v) and set to
    /// either 0xFF or 0x00 to emulate the PPU 1-bit latch in a way
    /// that is easy to shift each pixel.
    attribute_lsb_shift_register: u16,
}

///
/// Picture processing unit.
/// 
pub struct Ppu {
    /// Overall PPU cycle counter.
    total_cycle_count: u64,

    /// PPU Registers.
    reg: PpuRegisters,

    /// Current pointer position into VRAM.
    //vram_ptr: u16,

    /// PPU memory (vram, etc.)
    mem: Memory,

    /// Object Attribute Memory
    oam: Memory,

    //ppuaddr_latch: Latch,

    /// Reads to 2007 are done via an internal ppu buffer.
    /// A read returns the buffer contents and then the buffer
    /// is loaded with the value at PPUADDR
    ppudata_read_buffer: u8,

    //ppuscroll_latch: Latch,
    //ppuscroll_x_offset: u8,
    //ppuscroll_y_offset: u8,

    /// Frame counter
    frame: u64,

    /// PPU Rendering state, current scanline.
    scanline: u16,

    /// PPU Rendering state, scanline cycle.
    scanline_cycle: u16,

    bg_render_state: PpuBgRenderState,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            total_cycle_count: 0,
            reg: PpuRegisters::default(),
            //vram_ptr: 0,
            mem: Memory::new_ppu(),
            oam: Memory::new(OAM_SIZE),
            //ppuaddr_latch: Latch::Clear,
            ppudata_read_buffer: 0,
            //ppuscroll_latch: Latch::Clear,
            //ppuscroll_x_offset: 0,
            //ppuscroll_y_offset: 0,
            frame: 0,
            scanline: 261, // Start on prerender scanline
            scanline_cycle: 0,
            bg_render_state: PpuBgRenderState::default(),
        }
    }

    fn set_next_cycle(&mut self) {
        self.scanline_cycle += 1;

        // 262 scanlines x 341 pixels

        // Reset at cycle index 341
        if self.scanline_cycle > 340 {
            self.scanline_cycle = 0;
            self.scanline += 1;

            // Reset at scanline index 262
            if self.scanline > 261 {
                self.scanline = 0;
                self.frame += 1;

                // On odd numbered frames skip the idle tick on scanline 0, cycle 0
                if self.frame % 2 == 1 {
                    self.scanline_cycle = 1;
                }
            }
        }
    }

    pub fn cycle(&mut self) -> PpuCycleResult {
        self.total_cycle_count += 1;

        let result: PpuCycleResult = match self.scanline {
            0..=239 => {  // Visible scanlines

                //
                // DEBUG stuff
                //
                //if !self.reg.ppu_mask.render_bg {
                //    self.set_next_cycle();
                //    return PpuCycleResult::Idle;
                //}

                let cycle_result = match self.scanline_cycle {
                    0 => {
                        self.bg_render_state.fetch_state = PpuBgFetchState::BackgroundLSBAddr;
                        self.do_bg_fetches();
                        PpuCycleResult::Idle
                    }
                    1..=256 => {
                        if self.scanline_cycle == 1 {
                            self.bg_render_state.fetch_state = PpuBgFetchState::NametableAddr;
                        }

                        let pixel = self.render_bg_pixel();

                        self.do_bg_fetches();

                        if self.scanline_cycle % 8 == 0 {
                            self.update_bg_shift_registers();
                            self.update_coarse_x();
                        }
                        self.bg_render_state.fetch_state.next();

                        if self.scanline_cycle == 256 {
                            self.update_fine_y();
                        }

                        pixel
                    }
                    257 => {
                        self.reset_horizontal_position_in_v();

                        self.bg_render_state.fetch_state = PpuBgFetchState::Idle;
                        PpuCycleResult::HBlank { scanline: self.scanline, cycle: self.scanline_cycle }
                    }
                    258..=320 => {
                        self.bg_render_state.fetch_state = PpuBgFetchState::Idle;
                        PpuCycleResult::HBlank { scanline: self.scanline, cycle: self.scanline_cycle }
                    }
                    321..=336 => {
                        if self.scanline_cycle == 321 {
                            self.bg_render_state.fetch_state = PpuBgFetchState::NametableAddr;
                        }

                        self.do_bg_fetches();

                        if self.scanline_cycle % 8 == 0 {
                            self.update_bg_shift_registers();
                            self.update_coarse_x();
                        }

                        self.bg_render_state.fetch_state.next();
                        PpuCycleResult::HBlank { scanline: self.scanline, cycle: self.scanline_cycle }
                    }
                    337 | 339 => {
                        self.bg_render_state.fetch_state = PpuBgFetchState::NametableAddr;
                        self.do_bg_fetches();
                        self.bg_render_state.fetch_state.next();
                        PpuCycleResult::HBlank { scanline: self.scanline, cycle: self.scanline_cycle }
                    }
                    338 | 340 => {
                        self.do_bg_fetches();
                        PpuCycleResult::HBlank { scanline: self.scanline, cycle: self.scanline_cycle }
                    }
                    _ => panic!("invalid scanline/cycle: {}/{}", self.scanline, self.scanline_cycle)
                };

                cycle_result
            },
            240 => PpuCycleResult::PostRenderLine,
            241 => {
                if self.scanline_cycle == 1 {
                    self.set_vblank_flag();
                    let do_nmi = self.reg.ppu_ctrl.generate_nmi;
                    //let do_nmi = true;
                    PpuCycleResult::VBlankLine { trigger_nmi: do_nmi, scanline: self.scanline }
                } else {
                    PpuCycleResult::VBlankLine { trigger_nmi: false, scanline: self.scanline }
                }
            }
            242..=260 => PpuCycleResult::VBlankLine { trigger_nmi: false, scanline: self.scanline },
            261 => {
                match self.scanline_cycle {
                    0 => (),
                    1 => {
                        self.bg_render_state.fetch_state = PpuBgFetchState::NametableAddr;
                        self.clear_vblank_and_sprite_overflow();
                    },
                    2..=256 => (),
                    257 => {
                        self.reset_horizontal_position_in_v();
                    },
                    258..=279 => (),
                    280..=304 => {
                        self.reset_vertical_position_in_v();
                    }
                    305..=320 => (),
                    //
                    // TODO: There is some duplication here with the visible scanline cycles.
                    //       Refactor.
                    321..=336 => {
                        if self.scanline_cycle == 321 {
                            self.bg_render_state.fetch_state = PpuBgFetchState::NametableAddr;
                        }

                        self.do_bg_fetches();

                        if self.scanline_cycle % 8 == 0 {
                            self.update_bg_shift_registers();
                            self.update_coarse_x();
                        }

                        self.bg_render_state.fetch_state.next();
                    }
                    337 | 339 => {
                        self.bg_render_state.fetch_state = PpuBgFetchState::NametableAddr;
                        self.do_bg_fetches();
                        self.bg_render_state.fetch_state.next();
                    }
                    338 | 340 => {
                        self.do_bg_fetches();
                    }
                    _ => panic!("invalid scanline/cycle: {}/{}", self.scanline, self.scanline_cycle)
                };

                PpuCycleResult::PreRenderLine
            },
            _ => panic!("Invalid scanline: {}", self.scanline)
        };

        self.set_next_cycle();

        result
    }

    fn do_bg_fetches(&mut self) {
        //
        // See: https://www.nesdev.org/wiki/PPU_scrolling#Tile_and_attribute_fetching
        // for details on deducing tile/attribute addresses.
        //
        match self.bg_render_state.fetch_state {
            PpuBgFetchState::Idle => (),
            PpuBgFetchState::NametableAddr => {
                self.bg_render_state.tile_addr = 0x2000 | (self.reg.v & 0x0FFF)
            }
            PpuBgFetchState::NametableRead => {
                self.bg_render_state.tile_value =
                    self.mem.read(self.bg_render_state.tile_addr);
            }
            PpuBgFetchState::AttrtableAddr => {
                let v = self.reg.v;
                self.bg_render_state.attribute_addr = 
                    0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
            }
            PpuBgFetchState::AttrtableRead => {
                self.bg_render_state.attribute_data = self.mem.read(self.bg_render_state.attribute_addr);
            }
            // DCBA98 76543210
            // ---------------
            // 0HNNNN NNNNPyyy
            // |||||| |||||+++- T: Fine Y offset, the row number within a tile
            // |||||| ||||+---- P: Bit plane (0: less significant bit; 1: more significant bit)
            // ||++++-++++----- N: Tile number from name table
            // |+-------------- H: Half of pattern table (0: "left"; 1: "right")
            // +--------------- 0: Pattern table is at $0000-$1FFF
            PpuBgFetchState::BackgroundLSBAddr => {
                let fine_y = self.get_fine_y_scroll();

                self.bg_render_state.bg_lsb_addr = self.reg.ppu_ctrl.bg_pt_addr | 
                              ((self.bg_render_state.tile_value as u16) << 4) | fine_y as u16;
            }
            PpuBgFetchState::BackgroundLSBRead => {
                self.bg_render_state.bg_lsb = self.mem.read(self.bg_render_state.bg_lsb_addr);
            }
            PpuBgFetchState::BackgroundMSBAddr => {
                let fine_y = 0x8 | self.get_fine_y_scroll(); // Or with 0x8 for msb bit plane

                self.bg_render_state.bg_msb_addr = self.reg.ppu_ctrl.bg_pt_addr | 
                              ((self.bg_render_state.tile_value as u16) << 4) | fine_y as u16;
            }
            PpuBgFetchState::BackgroundMSBRead => {
                self.bg_render_state.bg_msb = self.mem.read(self.bg_render_state.bg_msb_addr);
            }
        }
    }

    fn update_bg_shift_registers(&mut self) {
        if matches!(self.bg_render_state.fetch_state, PpuBgFetchState::BackgroundMSBRead) {
            trace!("ppu: updating bg shift registers on scanline: {}, cycle: {}",
                    self.scanline, self.scanline_cycle);
            // last thing we did was read the BG pattern table MSB,
            // so update the shift registers
            self.bg_render_state.pattern_tile_lsb_register.push_byte(
                self.bg_render_state.bg_lsb
            );
            self.bg_render_state.pattern_tile_msb_register.push_byte(
                self.bg_render_state.bg_msb
            );

            // Fill attribute msb shift register with bit 1 of coarse Y
            // Lower 8 bits get filled with the bit.
            if utils::bit_is_set_u16(6, self.reg.v) {
                set_bits_from_mask_u16(0xFFFF, 0x00FF,
                &mut self.bg_render_state.attribute_msb_shift_register);
            }
            else {
                set_bits_from_mask_u16(0xFF00, 0x00FF,
                &mut self.bg_render_state.attribute_msb_shift_register);
            }

            // Fill attribute lsb shift register with bit 1 of coarse X
            // Lower 8 bits get filled with the bit.
            if utils::bit_is_set_u16(1, self.reg.v) {
                set_bits_from_mask_u16(0xFFFF, 0x00FF,
                &mut self.bg_render_state.attribute_lsb_shift_register);
            }
            else {
                set_bits_from_mask_u16(0xFF00, 0x00FF,
                &mut self.bg_render_state.attribute_lsb_shift_register);
            }
        }
        else {
            panic!("background fetch state out of sync with scanline cycle {}, {}, {:?}",
                self.scanline, self.scanline_cycle, self.bg_render_state.fetch_state);
        }
    }

    fn update_coarse_x(&mut self) {
        if self.reg.ppu_mask.render_bg {
            if (self.reg.v & 0x001F) == 31 { // if coarse X == 31
                self.reg.v &= !0x001F;       // coarse X = 0
                self.reg.v ^= 0x0400;        // switch horizontal nametable
            }
            else {
                self.reg.v += 1;             // increment coarse X
            }
        }
    }

    fn update_fine_y(&mut self) {
        if self.reg.ppu_mask.render_bg {
            if (self.reg.v & 0x7000) != 0x7000 {  // if fine Y < 7
              self.reg.v += 0x1000;               // increment fine Y
            }
            else {
              self.reg.v &= !0x7000;                   // fine Y = 0
              let mut y = (self.reg.v & 0x03E0) >> 5; // let y = coarse Y
              if y == 29 {
                  y = 0;                               // coarse Y = 0
                  self.reg.v ^= 0x0800;                // switch vertical nametable
              }
              else if y == 31 {
                  y = 0;                          // coarse Y = 0, nametable not switched
              }
              else {
                  y += 1;                         // increment coarse Y
              }
              self.reg.v = (self.reg.v & !0x03E0) | (y << 5);     // put coarse Y back into v
            }
        }
    }

    fn reset_horizontal_position_in_v(&mut self) {
        if self.reg.ppu_mask.render_bg {
            /*
              Reset horizontal position in v from t:
              v: ....A.. ...BCDEF <- t: ....A.. ...BCDEF
            */
            set_bits_from_mask_u16(self.reg.t, 0x041F, &mut self.reg.v);
        }
    }

    fn reset_vertical_position_in_v(&mut self) {
        if self.reg.ppu_mask.render_bg {
            /* Set vertical position in v from t:
               v: GHIA.BC DEF..... <- t: GHIA.BC DEF.....
            */
            set_bits_from_mask_u16(self.reg.t, 0x7BE0, &mut self.reg.v);
        }
    }

    fn render_bg_pixel(&mut self) -> PpuCycleResult {
        // TODO: Needs implementation
        if self.reg.ppu_mask.render_bg {
            PpuCycleResult::Pixel { scanline: self.scanline, x: self.scanline_cycle, color: 0 }
        } else {
            PpuCycleResult::Idle
        }
    }

    fn clear_vblank_and_sprite_overflow(&mut self) {
        clear_bit(7, &mut self.reg.ppu_status);
        clear_bit(5, &mut self.reg.ppu_status);
    }

    fn set_vblank_flag(&mut self) {
        set_bit(7, &mut self.reg.ppu_status);
    }

    /// Write to ppuctrl register.
    pub fn write_2000_ppuctrl(&mut self, value: u8) {
        if self.total_cycle_count < 30000 {
            return;
        }

        self.reg.ppu_ctrl.update(value);

        // Write nametable selection bits to t
        let nt_bits = (value as u16) << 10; 
        set_bits_from_mask_u16(nt_bits, 0xc00, &mut self.reg.t);
    }

    /// Write to ppumask register.
    pub fn write_2001_ppumask(&mut self, value: u8) {
        self.reg.ppu_mask.update(value);
    }

    /// Read and return ppustatus. Various side effects occur from reading this
    /// register. See nesdev.org PPU Registers page for info.
    pub fn read_2002_ppustatus(&mut self) -> u8 {
        let return_status = self.reg.ppu_status;

        //println!("READING PPU STATUS");

        // Reading the status register clears bit 7
        utils::clear_bit(7, &mut self.reg.ppu_status);

        // Reset address latch / write flag used by ppuaddr and ppuscroll
        self.reg.w = Toggle::FirstWrite;
        //self.ppuaddr_latch = Latch::Clear;
        //self.ppuscroll_latch = Latch::Clear;

        return_status
    }

    /// Write to oamaddr register.
    pub fn write_2003_oamaddr(&mut self, value: u8) {
        self.reg.oam_addr = value;
    }

    pub fn read_2004_oamdata(&self) -> u8 {
        self.oam.read(self.reg.oam_addr as u16)
    }

    pub fn write_2004_oamdata(&mut self, value: u8) {
        self.oam.write(self.reg.oam_addr as u16, value);
        self.reg.oam_addr = self.reg.oam_addr.wrapping_add(1);
    }

    pub fn write_2005_ppuscroll(&mut self, value: u8) {
        match self.reg.w {
            Toggle::FirstWrite => {
                // t: ....... ...ABCDE <- d: ABCDE...  coarse X scroll value
                let value_u16 = value as u16;
                set_bits_from_mask_u16(value_u16 >> 3, 0x1F, &mut self.reg.t);

                // x:              FGH <- d: .....FGH  fine X scroll value
                self.reg.x = value & 0x7;

                self.reg.w.toggle();
            }
            Toggle::SecondWrite => {
                // t: FGH..AB CDE..... <- d: ABCDEFGH
                let fgh = (value as u16) << 12;  // fine Y scroll
                let abcde = (value as u16) << 2; // coarse Y scroll

                set_bits_from_mask_u16(fgh, 0x7000, &mut self.reg.t);
                set_bits_from_mask_u16(abcde, 0x3E0, &mut self.reg.t);

                self.reg.w.toggle();
            }
        }
    }

    pub fn write_2006_ppuaddr(&mut self, value: u8) {
        match self.reg.w {

            Toggle::FirstWrite => {
                // t: .CDEFGH ........ <- d: ..CDEFGH
                //        <unused>     <- d: AB......
                // t: Z...... ........ <- 0 (bit Z is cleared)
                let mut value_u16 = (value & 0x3F) as u16; // and with 0x3F to clear 2 MSB's
                value_u16 = value_u16 << 8;

                // Using 0xFF00 mask here to clear the upper two bits of t since we cleared
                // them when initializing value_u16.
                set_bits_from_mask_u16(value_u16, 0xFF00, &mut self.reg.t);

                self.reg.w.toggle();
            }
            Toggle::SecondWrite => {
                // t: ....... ABCDEFGH <- d: ABCDEFGH
                // v: <...all bits...> <- t: <...all bits...>
                // w:                  <- 0
                set_bits_from_mask_u16(value as u16, 0xFF, &mut self.reg.t);

                self.reg.t = self.reg.t & 0x3FFF; // mirror down beyond 0x3FFF
                self.reg.v = self.reg.t;

                self.reg.w.toggle();

                //println!("t: {:04X}, v: {:04X}", self.reg.t, self.reg.v);
            }
        }
        
        // Old implementation. Cleaner, but not as accurate.
        //match self.ppuaddr_latch {
        //    Latch::Clear => self.ppuaddr_latch = Latch::Set(value),
        //    Latch::Set(msb) => {
        //        let addr = u16::from_le_bytes([value, msb]);
        //        self.reg.ppu_addr = addr & 0x3FFF; // mirror down beyond 0x3fff
        //        self.ppuaddr_latch = Latch::Clear;
        //    }
        //}
    }

    pub fn write_2007_ppudata(&mut self, value: u8) {
        self.mem.write(self.reg.v, value);
        self.reg.v += self.reg.ppu_ctrl.vram_increment;
        //println!("t: {:04X}, v: {:04X}", self.reg.t, self.reg.v);
    }

    pub fn read_2007_ppudata(&mut self) -> u8 {
        if self.reg.v > 0x3EFF {
            let value = self.mem.read(self.reg.v);
            self.reg.v += self.reg.ppu_ctrl.vram_increment;

            value
        } else {
            let value = self.ppudata_read_buffer;

            self.reg.v += self.reg.ppu_ctrl.vram_increment;
            self.ppudata_read_buffer = self.mem.read(self.reg.v);

            value
        }
    }

    pub fn oam_dma(&mut self, data: &[u8]) {
        self.oam.load(0, data);
    }

    pub fn mem_load(&mut self, addr: u16, data: &[u8]) {
        self.mem.load(addr, data);
    }

    fn get_fine_y_scroll(&self) -> u8 {
        ((self.reg.t & 0x7000) >> 12) as u8
    }

}