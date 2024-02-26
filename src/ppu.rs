use crate::utils::{self, set_bits_from_mask_u16};
use crate::mem::Memory;

/// Size of Object Attribute Memory.
const OAM_SIZE: usize = 256;

pub enum PpuCycleResult {
    Idle,
    Pixel {scanline: u16, x: u16, color: u8},
    HBlank {scanline: u16},
    PostRenderLine,
    VBlankLine {scanline: u16},
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
    ppu_mask: u8,

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

//impl Default for PpuFetchState {
//    fn default() -> Self {
//        PpuFetchState::Idle
//    }
//}

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
struct PpuRenderState {
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

    /// PPU Rendering state, current scanline.
    scanline: u16,

    /// PPU Rendering state, scanline cycle.
    cycle: u16,

    render_state: PpuRenderState,
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
            scanline: 261, // Start on prerender scanline
            cycle: 0,
            render_state: PpuRenderState::default(),
        }
    }

    fn set_next_cycle(&mut self) {
        self.cycle += 1;

        // 262 scanlines x 341 pixels

        // Reset at cycle index 341
        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            // Reset at scanline index 262
            if self.scanline > 261 {
                self.scanline = 0;
            }
        }
    }

    pub fn cycle(&mut self) -> PpuCycleResult {
        self.total_cycle_count += 1;
        
        let result: PpuCycleResult = match self.scanline {
            0..=239 => {  // Visible scanlines
                self.do_bg_fetches();
                self.render_pixel()
            },
            261 => {

                PpuCycleResult::PreRenderLine
            },
            _ => PpuCycleResult::Idle,
        };

        self.set_next_cycle();

        result
    }

    fn do_bg_fetches(&mut self) {

        if self.cycle == 0 {
            self.render_state.fetch_state = PpuBgFetchState::NametableAddr;
            return;
        }

        //
        // See: https://www.nesdev.org/wiki/PPU_scrolling#Tile_and_attribute_fetching
        // for details on deducing tile/attribute addresses.
        //
        match self.render_state.fetch_state {
            PpuBgFetchState::Idle => return,
            PpuBgFetchState::NametableAddr => {
                self.render_state.tile_addr = 0x2000 | (self.reg.v & 0x0FFF)
            }
            PpuBgFetchState::NametableRead => {
                self.render_state.tile_value =
                    self.mem.read(self.render_state.tile_addr);
            }
            PpuBgFetchState::AttrtableAddr => {
                let v = self.reg.v;
                self.render_state.attribute_addr = 
                    0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
            }
            PpuBgFetchState::AttrtableRead => {
                self.render_state.attribute_data = self.mem.read(self.render_state.attribute_addr);
                self.reg.v += self.reg.ppu_ctrl.vram_increment;
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

                self.render_state.bg_lsb_addr = self.reg.ppu_ctrl.bg_pt_addr | 
                              ((self.render_state.tile_value as u16) << 4) | fine_y as u16;
            }
            PpuBgFetchState::BackgroundLSBRead => {
                self.render_state.bg_lsb = self.mem.read(self.render_state.bg_lsb_addr);
            }
            PpuBgFetchState::BackgroundMSBAddr => {
                let fine_y = 0x8 | self.get_fine_y_scroll(); // Or with 0x8 for msb bit plane

                self.render_state.bg_msb_addr = self.reg.ppu_ctrl.bg_pt_addr | 
                              ((self.render_state.tile_value as u16) << 4) | fine_y as u16;
            }
            PpuBgFetchState::BackgroundMSBRead => {
                self.render_state.bg_msb = self.mem.read(self.render_state.bg_msb_addr);
            }
        }
    }

    fn render_pixel(&mut self) -> PpuCycleResult {
        // Needs implementation
        PpuCycleResult::Pixel { scanline: self.scanline, x: self.cycle, color: 0 }
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
        self.reg.ppu_mask = value;
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