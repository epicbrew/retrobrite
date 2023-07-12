use crate::mem::*;

///
/// This structure contains state data that needs to be accessed by multiple
/// components plus metadata about events (like read/write) to memory mapped
/// registers. This structure is passed to the CPU, Mapper, PPU, and APU
/// when it's their turn to cycle.
/// 
pub struct NesState {
    pub cpu_mem: Memory,
    ppu_mem: Memory,
}

impl NesState {
    pub fn new() -> Self {
        Self {
            cpu_mem: Memory::default(),
            ppu_mem: Memory::default(),
        }
    }
}