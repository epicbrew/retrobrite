use super::Mapper;
use crate::mem::NesState;
use crate::ines;


pub struct NromMapper {
    name: &'static str,
    number: u16,
}

impl Mapper for NromMapper {
    fn new() -> Self {
        Self {
            name: "NROM",
            number: 0,
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn number(&self) -> u16 {
        self.number
    }

    fn load_rom(&mut self, mc: &mut NesState, ines: &ines::InesRom) {
        mc.cpu_mem_load(0x8000, &ines.prg_rom);
    
        if ines.header.num_prg_rom_chunks == 1 {
            mc.cpu_mem_load(0xC000, &ines.prg_rom);
        }

        mc.ppu_mem_load(0x0000, &ines.chr_rom);

        //match ines.header.mirroring {
        //    ines::Mirroring::Horizontal => todo!(),
        //    ines::Mirroring::Vertical => todo!(),
        //}
    }

    fn cycle_to(&mut self, _mc: &mut NesState, _cycle: u64) {
        // NROM doesn't do anything
    }
}
