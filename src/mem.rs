use crate::ppu::constants::*;

//const PAGE_SIZE: u16 = 256;
//const MEMORY_SIZE: usize = 1024 * 64;

const CPU_MEMORY_SIZE: usize = 1024 * 64;
const PPU_MEMORY_SIZE: usize = 1024 * 16;
const VRAM_BANK_SIZE: usize = 1024; // PPU has 2K of VRAM, so two of these banks

/// Struct representing NES memory.
/// Used for CPU memory.
pub struct Memory {
    mem: Vec<u8>,
}

impl Memory {
    /// Creates a new memory object of the specified size.
    pub fn new(size: usize) -> Self {
        Self {
            mem: vec![0u8; size],
        }
    }

    /// Creates 64KB CPU memory.
    pub fn new_cpu() -> Self {
        Self {
            mem: vec![0u8; CPU_MEMORY_SIZE],
        }
    }

    /// Read an 8-bit value from memory.
    pub fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    /// Read a 16-bit value from memory.
    #[allow(dead_code)]
    pub fn read_word(&self, addr: u16) -> u16 {
        let addr = addr as usize;

        let lsb = self.mem[addr] as u16;
        let msb = self.mem[addr + 1] as u16;

        msb << 8 | lsb
    }

    /// Write an 8-bit value to memory.
    pub fn write(&mut self, addr: u16, value: u8) {
        //println!("write addr: {:04X}, {}", addr, addr);
        self.mem[addr as usize] = value;
    }

    /// Load a sequence of bytes into memory, starting at addr.
    pub fn load(&mut self, addr: u16, data: &[u8]) {
        if addr as usize + data.len() > self.mem.len() {
            panic!("load data would exceed memory capacity");
        }

        for (offset, value) in data.iter().enumerate() {
            let location = addr as usize + offset;
            self.mem[location] = *value;
        }
    }

    pub fn wrapping_load(&mut self, addr: u16, data: &[u8]) {
        let mut location = addr as usize;
        for value in data.iter() {
            self.mem[location] = *value;

            location += 1;

            if location == self.mem.len() {
                location = 0;
            }
        }
    }

    pub fn get_slice(&self, start_addr: u16, size: usize) -> &[u8] {
        let start = start_addr as usize;
        let end = start + size;

        &self.mem[start .. end]
    }
}


/// Struct representing NES memory.
/// Used for PPU memory.
/// Note that nametable addresses in field 'mem' between 0x2000 and 0x3000
/// are never actually written to in this implementation. Instead they are
/// mapped into the appropriate PPU internal vram banks.
pub struct PpuMemory {
    mem: Vec<u8>,
    vram_bank0: Vec<u8>,
    vram_bank1: Vec<u8>,
    mirroring: Mirroring,
}

impl PpuMemory {

    /// Creates 16KB PPU memory.
    pub fn new() -> Self {
        Self {
            mem: vec![0u8; PPU_MEMORY_SIZE],
            vram_bank0: vec![0u8; VRAM_BANK_SIZE],
            vram_bank1: vec![0u8; VRAM_BANK_SIZE],
            // Choosing any initial value for mirroring. Mapper will change it
            // during startup.
            mirroring: Mirroring::Vertical,
        }
    }

    pub fn set_mirroring(&mut self, mode: Mirroring) {
        self.mirroring = mode;
    }

    /// Utility function for mappers that ensures ppu address is properly wrapped
    /// at 0x4000 and mirrored.
    fn get_effective_address(&self, addr: u16) -> u16 {
        let effective_addr = addr % 0x4000; // Wrap at 0x4000
    
        match effective_addr {
            // Mirror down above 0x3F20
            0x3F20..=0x3FFF => effective_addr & 0x3F1F,
            _ => effective_addr
        }
    }

    /// Read an 8-bit value from memory.
    pub fn read(&self, addr: u16) -> u8 {
        let addr = self.get_effective_address(addr);

        match addr {
            NAMETABLE_0..=NAMETABLE_3_END => self.read_nametable(addr),
            _ => self.mem[addr as usize],
        }
    }

    /// Write an 8-bit value to memory.
    pub fn write(&mut self, addr: u16, value: u8) {
        //println!("write addr: {:04X}, {}", addr, addr);
        let addr = self.get_effective_address(addr);

        match addr {
            NAMETABLE_0..=NAMETABLE_3_END => self.write_nametable(addr, value),
            0x3F00 | 0x3F10 => {
                self.mem[0x3F00] = value;
                self.mem[0x3F10] = value;
            },
            0x3F04 | 0x3F14 => {
                self.mem[0x3F04] = value;
                self.mem[0x3F14] = value;
            },
            0x3F08 | 0x3F18 => {
                self.mem[0x3F08] = value;
                self.mem[0x3F18] = value;
            },
            0x3F0C | 0x3F1C => {
                self.mem[0x3F0C] = value;
                self.mem[0x3F1C] = value;
            },
            _ => {
                self.mem[addr as usize] = value;
            }
        }
    }

    fn read_nametable(&self, addr: u16) -> u8 {
        match addr {
            NAMETABLE_0..=NAMETABLE_0_END => {
                let vram_addr = (addr - NAMETABLE_0) as usize;
                match self.mirroring {
                    Mirroring::Vertical | Mirroring::Horizontal => self.vram_bank0[vram_addr],
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr],
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr],
                }
            },
            NAMETABLE_1..=NAMETABLE_1_END => {
                let vram_addr = (addr - NAMETABLE_1) as usize;
                match self.mirroring {
                    Mirroring::Vertical => self.vram_bank1[vram_addr],
                    Mirroring::Horizontal => self.vram_bank0[vram_addr],
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr],
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr],
                }
            },
            NAMETABLE_2..=NAMETABLE_2_END => {
                let vram_addr = (addr - NAMETABLE_2) as usize;
                match self.mirroring {
                    Mirroring::Vertical => self.vram_bank0[vram_addr],
                    Mirroring::Horizontal => self.vram_bank1[vram_addr],
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr],
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr],
                }

            },
            NAMETABLE_3..=NAMETABLE_3_END => {
                let vram_addr = (addr - NAMETABLE_3) as usize;
                match self.mirroring {
                    Mirroring::Vertical | Mirroring::Horizontal=> self.vram_bank1[vram_addr],
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr],
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr],
                }

            },
            _ => panic!("address is not a nametable address"),
        }
    }

    fn write_nametable(&mut self, addr: u16, value: u8) {
        match addr {
            NAMETABLE_0..=NAMETABLE_0_END => {
                let vram_addr = (addr - NAMETABLE_0) as usize;
                match self.mirroring {
                    Mirroring::Vertical | Mirroring::Horizontal => self.vram_bank0[vram_addr] = value,
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr] = value,
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr] = value,
                }
            },
            NAMETABLE_1..=NAMETABLE_1_END => {
                let vram_addr = (addr - NAMETABLE_1) as usize;
                match self.mirroring {
                    Mirroring::Vertical => self.vram_bank1[vram_addr] = value,
                    Mirroring::Horizontal => self.vram_bank0[vram_addr] = value,
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr] = value,
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr] = value,
                }
            },
            NAMETABLE_2..=NAMETABLE_2_END => {
                let vram_addr = (addr - NAMETABLE_2) as usize;
                match self.mirroring {
                    Mirroring::Vertical => self.vram_bank0[vram_addr] = value,
                    Mirroring::Horizontal => self.vram_bank1[vram_addr] = value,
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr] = value,
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr] = value,
                }

            },
            NAMETABLE_3..=NAMETABLE_3_END => {
                let vram_addr = (addr - NAMETABLE_3) as usize;
                match self.mirroring {
                    Mirroring::Vertical | Mirroring::Horizontal=> self.vram_bank1[vram_addr] = value,
                    Mirroring::OneScreen0 => self.vram_bank0[vram_addr] = value,
                    Mirroring::OneScreen1 => self.vram_bank1[vram_addr] = value,
                }

            },
            _ => panic!("address is not a nametable address"),
        }
    }
    /// Load a sequence of bytes into memory, starting at addr.
    pub fn load(&mut self, addr: u16, data: &[u8]) {
        if addr as usize + data.len() > self.mem.len() {
            panic!("load data would exceed memory capacity");
        }

        if addr >= NAMETABLE_0 && addr <= NAMETABLE_3_END {
            panic!("load does not implement nametable mirroring yet");
        }

        for (offset, value) in data.iter().enumerate() {
            let location = addr as usize + offset;
            self.mem[location] = *value;
        }
    }

    /*
    pub fn wrapping_load(&mut self, addr: u16, data: &[u8]) {
        let mut location = addr as usize;
        for value in data.iter() {
            self.mem[location] = *value;

            location += 1;

            if location == self.mem.len() {
                location = 0;
            }
        }
    }

    pub fn get_slice(&self, start_addr: u16, size: usize) -> &[u8] {
        let start = start_addr as usize;
        let end = start + size;

        &self.mem[start .. end]
    }
    */
}