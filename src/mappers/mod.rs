pub mod nrom000;

use ines::Ines;
use crate::mem::{MemController, MemObserver};


pub trait Mapper {
    fn new() -> Self;
    fn name(&self) -> &'static str;
    fn number(&self) -> u16;
    fn load_rom(&mut self, mc: &mut MemController, ines: &Ines);
    fn cycle_to(&mut self, mc: &mut MemController, cycle: u64);
    fn get_observer() -> Box<dyn MemObserver>;

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