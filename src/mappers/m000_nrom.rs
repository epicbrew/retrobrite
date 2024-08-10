use super::Mapper;
use crate::ines::{InesRom, MirroringType};
use crate::mem::{Memory, PpuMemory};
use crate::ppu::constants::*;


pub struct NromMapper {
    name: &'static str,
    number: u16,
    cpu_mem: Memory,
    ppu_mem: PpuMemory,
    mirroring: Mirroring,
    chr_ram: bool,
}

pub fn new(cpu_mem: Memory, ppu_mem: PpuMemory) -> NromMapper {
    NromMapper {
        name: "NROM",
        number: 0,
        cpu_mem,
        ppu_mem,
        mirroring: Mirroring::Horizontal,
        chr_ram: false,
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
        if ines.header.num_prg_rom_chunks < 1 || ines.header.num_prg_rom_chunks > 2 {
            panic!("nrom: invalid number of prg rom chunks");
        }

        self.cpu_mem.load(0x8000, &ines.prg_rom[0]);
    
        if ines.header.num_prg_rom_chunks == 1 {
            self.cpu_mem.load(0xC000, &ines.prg_rom[0]);
        } else {
            self.cpu_mem.load(0xC000, &ines.prg_rom[1]);
        }

        self.mirroring = match ines.header.flags6.mirroring {
            MirroringType::Horizontal => Mirroring::Horizontal,
            MirroringType::Vertical => Mirroring::Vertical,
        };

        self.ppu_mem.set_mirroring(self.mirroring);

        match ines.header.num_chr_rom_chunks {
            0 => self.chr_ram = true,
            1 => self.ppu_mem.load(0x0000, &ines.chr_rom[0]),
            _ => {
                panic!("nrom: invalid number of chr rom chunks: {}", ines.header.num_chr_rom_chunks);
            }
        }

    }
    
    fn cpu_read(&mut self, addr: u16) -> u8 {
        self.cpu_mem.read(addr)
    }
    
    fn cpu_write(&mut self, addr: u16, value: u8) {
        // Ignore writes to ROM space
        if addr < 0x8000 {
            self.cpu_mem.write(addr, value);
        }
    }
    
    fn get_cpu_dma_slice(&self, addr: u16) -> &[u8] {
        self.cpu_mem.get_slice(addr, 256)
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        self.ppu_mem.read(addr)
    }
    
    fn ppu_write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                if self.chr_ram {
                    self.ppu_mem.write(addr, value);
                }
                // Otherwise, cannot overwrite pattern table ROM
            }, 
            _ => self.ppu_mem.write(addr, value),
        }
    }

}
