use std::path::PathBuf;
use std::fs;

use crate::utils;

const DEFAULT_WRAM_SIZE: usize = 0x2000;

/// WRAM usually mapped to CPU memory at 0x6000-0x7FFF. This
/// is battery backed RAM for cartridges that feature persistent
/// save files.
pub struct WRam {
    mem: Vec<u8>,
    base_addr: u16,
    rom_name: String,
}

impl WRam {
    pub fn new(rom_name: &str) -> Self {
        let mut wram = Self { 
            mem: vec![0; DEFAULT_WRAM_SIZE],
            base_addr: 0x6000,
            rom_name: rom_name.to_string(),
         };

         wram.load_from_file();

         wram
    }

    fn load_from_file(&mut self) {
        let filepath = self.get_wram_filepath();

        if filepath.exists() {
            match fs::read(&filepath) {
                Ok(data) => {
                    self.mem = data;
                },
                Err(e) => {
                    println!("WARNING: could not open {}: {}", filepath.to_string_lossy(), e);
                }
            }
        }
    }

    pub fn write_to_file(&self) {
        let filepath = self.get_wram_filepath();
        match fs::write(&filepath, &self.mem) {
            Ok(_) => println!("wrote {}", filepath.to_string_lossy()),
            Err(e) => println!("faile to write {}: {}", filepath.to_string_lossy(), e),
        }
    }

    fn get_index(&self, addr: u16) -> usize {
        let index = addr - self.base_addr;

        index as usize
    }

    /// Read an 8-bit value from memory.
    pub fn read(&self, addr: u16) -> u8 {
        self.mem[self.get_index(addr)]
    }

    /// Write an 8-bit value to memory.
    pub fn write(&mut self, addr: u16, value: u8) {
        let index = self.get_index(addr);
        self.mem[index] = value;
    }

    /*
    /// Load a sequence of bytes into memory, starting at addr.
    pub fn load(&mut self, addr: u16, data: &[u8]) {
        if addr as usize + data.len() > self.mem.len() {
            panic!("load data would exceed memory capacity");
        }

        for (offset, value) in data.iter().enumerate() {
            let location = addr as usize + offset;
            self.mem[location] = *value;
        }
    }
    */

    /*
    fn open(&mut self, rom_name: String) {
        let mut data_dir = dirs::data_dir().unwrap();
        let wram_filename = rom_name + ".wram";
        data_dir.push("retrobrite");
        data_dir.push(wram_filename);

        
    }
    */

    fn get_wram_filepath(&self) -> PathBuf {
        /*
        let mut wram_filepath = dirs::data_dir().unwrap();
        let wram_filename = self.rom_name.to_string() + ".wram";
        wram_filepath.push("retrobrite");
        */

        let wram_filename = self.rom_name.to_string() + ".wram";
        let mut wram_filepath = utils::get_data_dir_path();
        wram_filepath.push(wram_filename);

        wram_filepath
    }
}