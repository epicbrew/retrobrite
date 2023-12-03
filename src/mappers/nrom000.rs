use super::Mapper;
use crate::state::NesState;
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

    fn load_rom(&mut self, state: &mut NesState, ines: &ines::InesRom) {
        state.cpu_mem_load(0x8000, &ines.prg_rom);
    
        if ines.header.num_prg_rom_chunks == 1 {
            state.cpu_mem_load(0xC000, &ines.prg_rom);
        }

        state.ppu_mem_load(0x0000, &ines.chr_rom);

        //match ines.header.mirroring {
        //    ines::Mirroring::Horizontal => todo!(),
        //    ines::Mirroring::Vertical => todo!(),
        //}
    }

    fn cycle_to(&mut self, _state: &mut NesState, _cycle: u64) {
        // NROM doesn't do anything
    }
}
