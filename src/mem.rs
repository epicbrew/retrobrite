
//const PAGE_SIZE: u16 = 256;
//const MEMORY_SIZE: usize = 1024 * 64;

const CPU_MEMORY_SIZE: usize = 1024 * 64;
const PPU_MEMORY_SIZE: usize = 1024 * 16;
//const VRAM_MEMORY_SIZE: usize = 1024 * 2;

/// Struct representing NES memory.
/// Used for CPU and PPU memory.
pub struct Memory {
    mem: Vec<u8>,
    is_ppu: bool,
}

impl Memory {
    /// Creates a new memory object of the specified size.
    pub fn new(size: usize) -> Self {
        Self {
            mem: vec![0u8; size],
            is_ppu: false,
        }
    }

    /// Creates 64KB CPU memory.
    pub fn new_cpu() -> Self {
        Self {
            mem: vec![0u8; CPU_MEMORY_SIZE],
            is_ppu: false,
        }
    }

    /// Creates 16KB PPU memory.
    pub fn new_ppu() -> Self {
        Self {
            mem: vec![0u8; PPU_MEMORY_SIZE],
            is_ppu: true,
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

        if self.is_ppu { // Do ppu palette mirroring
            match addr {
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

        } else {
            self.mem[addr as usize] = value;
        }
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

    pub fn get_slice(&self, start_addr: u16, size: usize) -> &[u8] {
        let start = start_addr as usize;
        let end = start + size;

        &self.mem[start .. end]
    }
}