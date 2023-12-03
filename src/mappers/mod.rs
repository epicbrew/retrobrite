pub mod nrom000;

use crate::ines::InesRom;
use crate::state::NesState;


pub trait Mapper {
    fn new() -> Self;
    fn name(&self) -> &'static str;
    fn number(&self) -> u16;
    fn load_rom(&mut self, state: &mut NesState, ines: &InesRom);
    fn cycle_to(&mut self, state: &mut NesState, cycle: u64);

    fn print_info(&self) {
        println!("Mapper: {} ({:03})", self.name(), self.number());
    }
}

pub fn get_mapper(number: u16) -> impl Mapper {
    match number {
        0 => nrom000::NromMapper::new(),
        _ => panic!("Unsupported mapper"),
    }
}