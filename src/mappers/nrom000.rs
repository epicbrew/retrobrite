use super::{Mapper, get_ppu_effective_address};
use crate::ines::{InesRom, MirroringType};
use crate::mem::Memory;
use crate::ppu::constants::*;


pub struct NromMapper {
    name: &'static str,
    number: u16,
    cpu_mem: Memory,
    ppu_mem: Memory,
    mirroring: MirroringType,
}

pub fn new(cpu_mem: Memory, ppu_mem: Memory) -> NromMapper {
    NromMapper {
        name: "NROM",
        number: 0,
        cpu_mem,
        ppu_mem,
        mirroring: MirroringType::Horizontal

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

        if ines.header.num_chr_rom_chunks != 1 {
            panic!("nrom: invalid number of chr rom chunks");
        }

        self.ppu_mem.load(0x0000, &ines.chr_rom[0]);

        self.mirroring = match ines.header.flags6.mirroring {
            MirroringType::Horizontal => MirroringType::Horizontal,
            MirroringType::Vertical => MirroringType::Vertical,
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
        let addr = get_ppu_effective_address(addr);
        self.ppu_mem.read(addr)
    }
    
    fn ppu_write(&mut self, addr: u16, value: u8) {
        let addr = get_ppu_effective_address(addr);

        match addr {
            0x0000..=0x1FFF => (), // Cannot overwrite pattern table ROM
            NAMETABLE_0..=NAMETABLE_3_END => {
                let mirrored_address = self.get_mirrored_address(addr);
                self.ppu_mem.write(addr, value);
                self.ppu_mem.write(mirrored_address, value);
            },
            _ => self.ppu_mem.write(addr, value),
        }
    }

}

impl NromMapper {
    fn get_mirrored_address(&self, addr: u16) -> u16 {
        match self.mirroring {
            MirroringType::Horizontal => {
                match addr {
                    NAMETABLE_0..=NAMETABLE_0_END | NAMETABLE_2..=NAMETABLE_2_END => {
                        addr + 0x400
                    },
                    NAMETABLE_1..=NAMETABLE_1_END | NAMETABLE_3..=NAMETABLE_3_END => {
                        addr - 0x400
                    },
                    _ => panic!("address is not within a nametable")
                }
            },
            MirroringType::Vertical => {
                match addr {
                    NAMETABLE_0..=NAMETABLE_0_END | NAMETABLE_1..=NAMETABLE_1_END => {
                        addr + 0x800
                    },
                    NAMETABLE_2..=NAMETABLE_2_END | NAMETABLE_3..=NAMETABLE_3_END => {
                        addr - 0x800
                    },
                    _ => panic!("address is not within a nametable")
                }
            }
        }
    }

}
