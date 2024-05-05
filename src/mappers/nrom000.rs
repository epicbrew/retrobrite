use super::{Mapper, get_ppu_effective_address};
use crate::ines::InesRom;
use crate::mem::Memory;


pub struct NromMapper {
    name: &'static str,
    number: u16,
    cpu_mem: Memory,
    ppu_mem: Memory
}

pub fn new(cpu_mem: Memory, ppu_mem: Memory) -> NromMapper {
    NromMapper {
        name: "NROM",
        number: 0,
        cpu_mem,
        ppu_mem
    }
}

impl Mapper for NromMapper {
    fn name(&self) -> &'static str {
        self.name
    }

    fn number(&self) -> u16 {
        self.number
    }

    fn load_rom(&mut self, ines: &InesRom) {
        self.cpu_mem.load(0x8000, &ines.prg_rom);
    
        if ines.header.num_prg_rom_chunks == 1 {
            self.cpu_mem.load(0xC000, &ines.prg_rom);
        }

        self.ppu_mem.load(0x0000, &ines.chr_rom);

        //match ines.header.mirroring {
        //    ines::Mirroring::Horizontal => todo!(),
        //    ines::Mirroring::Vertical => todo!(),
        //}
    }
    
    fn cpu_read(&mut self, addr: u16) -> u8 {
        self.cpu_mem.read(addr)
    }
    
    fn cpu_write(&mut self, addr: u16, value: u8) {
        // Ignore writes to ROM space
        //if addr < 0x8000 {
            self.cpu_mem.write(addr, value);
        //}
    }
    
    fn get_cpu_dma_slice(&self, addr: u16) -> &[u8] {
        self.cpu_mem.get_slice(addr, 256)
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        let addr = get_ppu_effective_address(addr);
        self.ppu_mem.read(addr)
    }
    
    fn ppu_write(&mut self, addr: u16, value: u8) {
        let addr = get_ppu_effective_address(addr);

        match addr {
            0x0000..=0x1FFF => (), // Cannot overwrite pattern table ROM
            _ => self.ppu_mem.write(addr, value),
        }
    }
}
