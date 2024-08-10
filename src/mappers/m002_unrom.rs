
use super::Mapper;
use crate::ines::{InesRom, MirroringType, PRG_ROM_CHUNK_SIZE};
use crate::mem::{Memory, PpuMemory};
use crate::ppu::constants::*;


pub struct UnromMapper {
    name: &'static str,
    number: u16,
    cpu_mem: Memory,
    ppu_mem: PpuMemory,
    mirroring: Mirroring,
    prg_rom_banks: Vec<[u8; PRG_ROM_CHUNK_SIZE]>
}

pub fn new(cpu_mem: Memory, ppu_mem: PpuMemory) -> UnromMapper {
    UnromMapper {
        name: "UNROM",
        number: 2,
        cpu_mem,
        ppu_mem,
        mirroring: Mirroring::Horizontal,
        prg_rom_banks: Vec::new()
    }
}

impl Mapper for UnromMapper {
    fn name(&self) -> &'static str {
        self.name
    }

    fn number(&self) -> u16 {
        self.number
    }

    fn load_rom(&mut self, ines: &InesRom) {
        self.init_prg_banks(&ines);

        self.cpu_mem.load(0x8000, &self.prg_rom_banks[0]);
        self.cpu_mem.load(0xC000, &self.prg_rom_banks[ines.header.num_prg_rom_chunks - 1]);

        // UNROM have chr ram, so no need to load anything into ppu mem here

        self.mirroring = match ines.header.flags6.mirroring {
            MirroringType::Horizontal => Mirroring::Horizontal,
            MirroringType::Vertical => Mirroring::Vertical,
        };

        self.ppu_mem.set_mirroring(self.mirroring);
    }
    
    fn cpu_read(&mut self, addr: u16) -> u8 {
        self.cpu_mem.read(addr)
    }
    
    fn cpu_write(&mut self, addr: u16, value: u8) {
        if addr < 0x8000 {
            self.cpu_mem.write(addr, value);
        }
        else {
            let bank = (value & 0x0F) as usize;
            //println!("bank switch to {}/{}, addr {}, mem val {}",
            //    bank, value, addr, self.cpu_mem.read(addr));
            self.cpu_mem.load(0x8000, &self.prg_rom_banks[bank]);
        }
    }
    
    fn get_cpu_dma_slice(&self, addr: u16) -> &[u8] {
        self.cpu_mem.get_slice(addr, 256)
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        self.ppu_mem.read(addr)
    }
    
    fn ppu_write(&mut self, addr: u16, value: u8) {
        self.ppu_mem.write(addr, value);
    }

}

impl UnromMapper {
    fn init_prg_banks(&mut self, ines: &InesRom) {
        for bank in ines.prg_rom.iter() {
            self.prg_rom_banks.push(*bank);
        }
    }
}
