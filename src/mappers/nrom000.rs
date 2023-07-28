use super::Mapper;
use crate::mem::{MemController, MemObserver};


pub struct NromMapper {
    name: &'static str,
    number: u16,
}

struct NromObserver;

impl MemObserver for NromObserver {
    fn read_happened(&mut self, _cycle: u64, _addr: u16) {
    }

    fn write_happened(&mut self, _cycle: u64, _addr: u16, _value: u8) {
    }
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

    fn load_rom(&mut self, mc: &mut MemController, ines: &ines::Ines) {
        mc.cpu_mem_load(0x8000, &ines.prg_rom);
    
        if ines.header.num_prg_rom_blocks == 1 {
            mc.cpu_mem_load(0xC000, &ines.prg_rom);
        }
    }

    fn cycle_to(&mut self, _mc: &mut MemController, _cycle: u64) {
        // NROM doesn't do anything
    }

    fn get_observer(&self) -> Box<dyn MemObserver> {
        Box::new(NromObserver)
    }
}
