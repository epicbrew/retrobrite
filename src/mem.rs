
//const PAGE_SIZE: u16 = 256;
const MEMORY_SIZE: usize = 1024 * 64;

pub struct Memory {
    mem: [u8; MEMORY_SIZE],
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            mem: [0u8; MEMORY_SIZE]
        }
    }
}

impl Memory {
    pub fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value;
    }
}