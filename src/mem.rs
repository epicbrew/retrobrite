
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
    /// Read an 8-bit value from memory.
    pub fn read(&self, addr: u16) -> u8 {
        let addr = self.get_effective_address(addr);
        self.mem[addr as usize]
    }

    /// Read a 16-bit value from memory.
    pub fn read_word(&self, addr: u16) -> u16 {
        let addr = self.get_effective_address(addr);
        let addr = addr as usize;

        let lsb = self.mem[addr] as u16;
        let msb = self.mem[addr + 1] as u16;

        msb << 8 | lsb
    }

    /// Write an 8-bit value to memory.
    pub fn write(&mut self, addr: u16, value: u8) {
        let addr = self.get_effective_address(addr);
        self.mem[addr as usize] = value;
    }

    /// Load a sequence of bytes into memory, starting at addr.
    pub fn load(&mut self, addr: u16, data: &[u8]) {
        let addr = self.get_effective_address(addr);

        if addr as usize + data.len() > self.mem.len() {
            panic!("load data would exceed memory capacity");
        }

        for (offset, value) in data.iter().enumerate() {
            let location = addr as usize + offset;
            self.mem[location] = *value;
        }
    }

    /// Converts mirrored addresses to their effective addresses when addr
    /// falls within one of the mirrored ranges.
    fn get_effective_address(&self, addr: u16) -> u16 {
        match addr {
            // 0000-07FF is RAM with 0800-1FFF mirroring it
            // (AND with 07FF to get effective address)
            0x0000..=0x1FFF => addr & 0x07FF,

            // 2000-2007 are PPU registers with 2008-3FFF mirroring it
            // (AND with 2007 to get effective address)
            0x2000..=0x3FFF => addr & 0x2007,

            // Everything else uses the originally provided address
            _ => addr
        }
    }
}