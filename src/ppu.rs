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
    ppu_ctrl: u8,

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
struct PpuFlags {
    _vblank: bool,
    _hblank: bool,
    _rendering: bool,
}
#[derive(Default)]
enum PpuFetchState {
    #[default] Idle,
    NametableAddr,
    NametableRead(u16),
    AttrtableAddr,
    AttrtableRead(u16),
    BackgroundLSBAddr,
    BackgroundLSBRead(u16),
    BackgroundMSBAddr,
    BackgroundMSBRead(u16)
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
    fetch_state: PpuFetchState,
    pattern_tile_msb_register: ShiftRegister16Bit,
    pattern_tile_lsb_register: ShiftRegister16Bit,
}

///
/// Picture processing unit.
/// 
pub struct Ppu {
    /// PPU flags for bookkeeping.
    _flags: PpuFlags,

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
            _flags: PpuFlags::default(),
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
                self.do_fetches();
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

    fn do_fetches(&mut self) {

        if self.cycle == 0 {
            self.render_state.fetch_state = PpuFetchState::NametableAddr;
            return;
        }
        // Needs implementation
        match self.render_state.fetch_state {
            PpuFetchState::Idle => return,
            PpuFetchState::NametableAddr => todo!(),
            PpuFetchState::NametableRead(addr) => todo!(),
            PpuFetchState::AttrtableAddr => todo!(),
            PpuFetchState::AttrtableRead(addr) => todo!(),
            PpuFetchState::BackgroundLSBAddr => todo!(),
            PpuFetchState::BackgroundLSBRead(addr) => todo!(),
            PpuFetchState::BackgroundMSBAddr => todo!(),
            PpuFetchState::BackgroundMSBRead(addr) => todo!(),
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

        self.reg.ppu_ctrl = value;

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
                let mut value_u16 = (value & 0x3F) as u16;
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
        self.reg.v += self.ppu_ctrl_get_vram_increment();
    }

    pub fn read_2007_ppudata(&mut self) -> u8 {
        if self.reg.v > 0x3EFF {
            let value = self.mem.read(self.reg.v);
            self.reg.v += self.ppu_ctrl_get_vram_increment();

            value
        } else {
            let value = self.ppudata_read_buffer;

            self.reg.v += self.ppu_ctrl_get_vram_increment();
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

    fn _ppu_ctrl_get_base_nametable_addr(&self) -> u16 {
        let index = self.reg.ppu_ctrl & 0x0003;
        match index {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!("ppu_ctrl_get_base_nametable_addr: invalid index")
        }
    }

    fn ppu_ctrl_get_vram_increment(&self) -> u16 {
        if utils::bit_is_set(2, self.reg.ppu_ctrl) {
            32
        } else {
            1
        }
    }
}