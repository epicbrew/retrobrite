pub mod m000_nrom;
pub mod m001_mmc1;
pub mod m002_unrom;

use crate::ines::InesRom;
use crate::mem::{Memory, PpuMemory};

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

    /// Perform any shutdown tasks (write wram file, etc).
    fn shutdown(&mut self) {
        // Default is to do nothing
    }

    /// Print mapper name/number to stdout.
    fn print_info(&self) {
        println!("Mapper: {} ({:03})", self.name(), self.number());
    }
}

/// Factory function to create a new mapper.
/// The passed in mapper number determines the type of mapper returned.
pub fn get_mapper(number: u16, cpu_mem: Memory, ppu_mem: PpuMemory) -> Box<dyn Mapper> {
    match number {
        //0 => Box::new(nrom000::NromMapper::new()),
        0 => Box::new(m000_nrom::new(cpu_mem, ppu_mem)),
        1 => Box::new(m001_mmc1::new(cpu_mem, ppu_mem)),
        2 => Box::new(m002_unrom::new(cpu_mem, ppu_mem)),
        71 => Box::new(m002_unrom::new(cpu_mem, ppu_mem)), // Same as mapper 002
        _ => panic!("Unsupported mapper: {}", number),
    }
}
