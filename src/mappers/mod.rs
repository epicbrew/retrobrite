pub mod nrom000;

use crate::ines::InesRom;
use crate::mem::Memory;

const NAMETABLE_0: u16 = 0x2000;

///
/// Trait for implementing a mapper.
/// Mappers can observe and react to all cpu and ppu memory reads/writes.
/// 
pub trait Mapper {
    /// Get the name of this mapper.
    fn name(&self) -> &'static str;

    /// Get this mapper's number/id.
    fn number(&self) -> u16;

    /// Load a rom from an ines file object into memory.
    fn load_rom(&mut self, ines: &InesRom);

    /// Read from CPU memory.
    fn cpu_read(&mut self, addr: u16) -> u8;

    /// Write to CPU memory.
    fn cpu_write(&mut self, addr: u16, value: u8);

    /// Returns a slice of CPU memory for using in OAM DMA.
    fn get_cpu_dma_slice(&self, addr: u16) -> &[u8];

    /// Read from PPU memory.
    fn ppu_read(&mut self, addr: u16) -> u8;

    /// Write to PPU memory.
    fn ppu_write(&mut self, addr: u16, value: u8);

    /// Print mapper name/number to stdout.
    fn print_info(&self) {
        println!("Mapper: {} ({:03})", self.name(), self.number());
    }
}

/// Factory function to create a new mapper.
/// The passed in mapper number determines the type of mapper returned.
pub fn get_mapper(number: u16, cpu_mem: Memory, ppu_mem: Memory) -> Box<dyn Mapper> {
    match number {
        //0 => Box::new(nrom000::NromMapper::new()),
        0 => Box::new(nrom000::new(cpu_mem, ppu_mem)),
        _ => panic!("Unsupported mapper"),
    }
}

/// Utility function for mappers that ensures ppu address is properly wrapped
/// at 0x4000 and mirrored.
pub fn get_ppu_effective_address(addr: u16) -> u16 {
    let effective_addr = addr % 0x4000; // Wrap at 0x4000

    match effective_addr {
        0x3F20..=0x3FFF => effective_addr & 0x3F1F,
        _ => effective_addr
    }
}
