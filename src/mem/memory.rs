
//const PAGE_SIZE: u16 = 256;
//const MEMORY_SIZE: usize = 1024 * 64;

const CPU_MEMORY_SIZE: usize = 1024 * 64;
const PPU_MEMORY_SIZE: usize = 1024 * 16;

/// Struct representing NES memory.
/// Used for CPU and PPU memory.
pub struct Memory {
    mem: Vec<u8>,
}

impl Memory {

    /// Creates 64KB CPU memory.
    pub fn new_cpu() -> Self {
        Self {
            mem: vec![0u8; CPU_MEMORY_SIZE],
        }
    }

    /// Creates 16KB PPU memory.
    pub fn new_ppu() -> Self {
        Self {
            mem: vec![0u8; PPU_MEMORY_SIZE],
        }
    }

    /// Read an 8-bit value from memory.
    pub fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    /// Read a 16-bit value from memory.
    pub fn read_word(&self, addr: u16) -> u16 {
        let addr = addr as usize;

        let lsb = self.mem[addr] as u16;
        let msb = self.mem[addr + 1] as u16;

        msb << 8 | lsb
    }

    /// Write an 8-bit value to memory.
    pub fn write(&mut self, addr: u16, value: u8) {
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
}