use crate::utils;
use crate::mem::memory::Memory;

/// Size of Object Attribute Memory.
const OAM_SIZE: usize = 256;

/// Latch for ppuaddr, ppuscroll, etc.
pub enum Latch {
    /// Latch state clear.
    Clear,

    /// Latch state set with value.
    Set(u8),
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
    /// OAM Data (Cpu: $2004, Bits: dddd dddd).
    /// OAM data read/write.
    /// 
    //oam_data: u8,

    //
    // PPU Scroll (Cpu: $2005, Bits: xxxx xxxx).
    // Fine scroll position (two writes: X scroll, Y scroll).
    // 
    //ppu_scroll: u8,

    ///
    /// PPU Address (Cpu: $2006, Bits: aaaa aaaa).
    /// PPU read/write address (two writes: most significant byte, least significant byte).
    /// 
    ppu_addr: u16,

    //
    // PPU Data (Cpu: $2007, Bits: dddd dddd).
    // PPU data read/write.
    // 
    //ppu_data: u8,

    //
    // OAM DMA (Cpu: $4014, Bits: aaaa aaaa).
    // OAM DMA high address.
    // 
    //oam_dma: u8,
}

#[derive(Default)]
struct PpuFlags {
    _vblank: bool,
    _hblank: bool,
    _rendering: bool,
}

///
/// Picture processing unit.
/// 
pub struct Ppu {
    /// PPU flags for bookkeeping.
    _flags: PpuFlags,

    /// PPU cycle counter.
    cycle_count: u64,

    /// PPU Registers.
    reg: PpuRegisters,

    /// Current pointer position into VRAM.
    //vram_ptr: u16,

    /// PPU memory (vram, etc.)
    mem: Memory,

    /// Object Attribute Memory
    oam: Memory,

    ppuaddr_latch: Latch,

    /// Reads to 2007 are done via an internal ppu buffer.
    /// A read returns the buffer contents and then the buffer
    /// is loaded with the value at PPUADDR
    ppudata_read_buffer: u8,

    ppuscroll_latch: Latch,
    ppuscroll_x_offset: u8,
    ppuscroll_y_offset: u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            _flags: PpuFlags::default(),
            cycle_count: 0,
            reg: PpuRegisters::default(),
            //vram_ptr: 0,
            mem: Memory::new_ppu(),
            oam: Memory::new(OAM_SIZE),
            ppuaddr_latch: Latch::Clear,
            ppudata_read_buffer: 0,
            ppuscroll_latch: Latch::Clear,
            ppuscroll_x_offset: 0,
            ppuscroll_y_offset: 0,
        }
    }

    /// Write to ppuctrl register.
    pub fn write_2000_ppuctrl(&mut self, value: u8) {
        if self.cycle_count < 30000 {
            return;
        }

        self.reg.ppu_ctrl = value;
    }

    /// Write to ppumask register.
    pub fn write_2001_ppumask(&mut self, value: u8) {
        self.reg.ppu_mask = value;
    }

    /// Read and return ppustatus. Various side effects occur from reading this
    /// register. See nesdev.org PPU Registers page for info.
    pub fn read_2002_ppustatus(&mut self) -> u8 {
        let return_status = self.reg.ppu_status;

        println!("READING PPU STATUS");

        // Reading the status register clears bit 7
        utils::clear_bit(7, &mut self.reg.ppu_status);

        // Reset address latches used by ppuaddr and ppuscroll
        self.ppuaddr_latch = Latch::Clear;
        self.ppuscroll_latch = Latch::Clear;

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
        match self.ppuscroll_latch {
            Latch::Clear => self.ppuscroll_latch = Latch::Set(value),
            Latch::Set(x_offset) => {
                self.ppuscroll_x_offset = x_offset;
                self.ppuscroll_y_offset = value;
                self.ppuscroll_latch = Latch::Clear;
            }
        }
    }

    pub fn write_2006_ppuaddr(&mut self, value: u8) {
        match self.ppuaddr_latch {
            Latch::Clear => self.ppuaddr_latch = Latch::Set(value),
            Latch::Set(msb) => {
                let addr = u16::from_le_bytes([value, msb]);
                self.reg.ppu_addr = addr & 0x3FFF; // mirror down beyond 0x3fff
                self.ppuaddr_latch = Latch::Clear;
            }
        }
    }

    pub fn write_2007_ppudata(&mut self, value: u8) {
        self.mem.write(self.reg.ppu_addr, value);
        self.reg.ppu_addr += self.ppu_ctrl_get_vram_increment();
    }

    pub fn read_2007_ppudata(&mut self) -> u8 {
        if self.reg.ppu_addr > 0x3EFF {
            let value = self.mem.read(self.reg.ppu_addr);
            self.reg.ppu_addr += self.ppu_ctrl_get_vram_increment();

            value
        } else {
            let value = self.ppudata_read_buffer;

            self.reg.ppu_addr += self.ppu_ctrl_get_vram_increment();
            self.ppudata_read_buffer = self.mem.read(self.reg.ppu_addr);

            value
        }
    }

    pub fn oam_dma(&mut self, data: &[u8]) {
        self.oam.load(0, data);
    }

    fn ppu_ctrl_get_base_nametable_addr(&self) -> u16 {
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