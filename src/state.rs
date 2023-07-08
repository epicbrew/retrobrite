use crate::mem::*;

///
/// This structure contains state data that needs to be accessed by multiple
/// components plus metadata about events (like read/write) to memory mapped
/// registers. This structure is passed to the CPU, Mapper, PPU, and APU
/// when it's their turn to cycle.
/// 
struct _NesSharedState {
    cpu_mem: Memory,
    ppu_mem: Memory,
}