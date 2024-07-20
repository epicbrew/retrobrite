use std::{cell::RefCell, rc::Rc};
use crate::ppu::Ppu;
use crate::mappers::Mapper;
use crate::utils;

///
/// Manages reads/writes to cpu and ppu memory, properly delegating calls to
/// ppu/apu functions when memory mapped registers are used.
/// 
pub struct NesState {
    mapper: Box<dyn Mapper>,
    ppu_ref: Rc<RefCell<Ppu>>,

    /// Controller shift register strobe state. If true we will accept controller
    /// state updates from the UI. If false the program is in the process of reading
    /// the controllers and we shouldn't reload the state.
    reload_controller_state: bool,

    /// Last received input state from the UI.
    controller1_state: u8,

    /// Shift register contents for controller 1
    controller1_register: u8,

    /// Read count since $4016 strobe was set to 0. We will return 1 for
    /// any shift register reads after the first eight until the strobe is
    /// reset.
    controller1_read_count: u16,

    /// Last received input state from the UI.
    controller2_state: u8,

    /// Shift register contents for controller 2
    controller2_register: u8,

    /// Read count since $4016 strobe was set to 0. We will return 1 for
    /// any shift register reads after the first eight until the strobe is
    /// reset.
    controller2_read_count: u16,
}

impl NesState {
    pub fn new(mapper: Box<dyn Mapper>, ppu_ref: Rc<RefCell<Ppu>>) -> Self {
        Self {
            mapper,
            ppu_ref,
            reload_controller_state: true,
            controller1_state: 0,
            controller1_register: 0,
            controller1_read_count: 0,
            controller2_state: 0,
            controller2_register: 0,
            controller2_read_count: 0,
        }
    }

    fn get_cpu_effective_address(&self, addr: u16) -> u16 {
        match addr {
            // 0000-07FF is RAM with 0800-1FFF mirroring it
            // (AND with 07FF to get effective address)
            0x0000..=0x1FFF => addr & 0x07FF,

            // 2000-2007 are PPU registers with 2008-3FFF mirroring it
            // (AND with 2007 to get effective address)
            0x2000..=0x3FFF => addr & 0x2007,

            // Everything else uses the originally provided address
            _ => addr
        }
    }

    /// Read from CPU memory and notify observers.
    pub fn cpu_mem_read(&mut self, _cycle: u64, addr: u16) -> u8 {
        let addr = self.get_cpu_effective_address(addr);

        let read_result = match addr {
            0x2002 => self.ppu_ref.borrow_mut().read_2002_ppustatus(),
            0x2004 => self.ppu_ref.borrow().read_2004_oamdata(),
            0x2007 => {
                self.ppu_ref.borrow_mut().read_2007_ppudata(&mut self.mapper)
            },
            0x4016 => self.read_controller1(),
            0x4017 => self.read_controller2(),
            _ => self.mapper.cpu_read(addr)
        };

        read_result
    }

    /// Read a 16-bit value from cpu memory.
    pub fn cpu_mem_read_word(&mut self, addr: u16) -> u16 {
        let addr = self.get_cpu_effective_address(addr);

        let lsb = self.mapper.cpu_read(addr) as u16;
        let msb = self.mapper.cpu_read(addr + 1) as u16;

        msb << 8 | lsb
    }
    
    /// Write an 8-bit value to memory, properly forwarding writes to PPU register
    /// ports as appropriate.
    pub fn cpu_mem_write(&mut self, addr: u16, value: u8, cycle_count: u64) {
        let addr = self.get_cpu_effective_address(addr);

        // PPU ignores certain writes until approx 29658 cpu cycles
        let ppu_ready = cycle_count > 29658;

        // Handle PPU register address writes if necessary.
        match addr {
            0x2000 => {
                if ppu_ready {
                    self.ppu_ref.borrow_mut().write_2000_ppuctrl(value);
                }
            },
            0x2001 => {
                if ppu_ready {
                    self.ppu_ref.borrow_mut().write_2001_ppumask(value);
                }
            },
            0x2003 => self.ppu_ref.borrow_mut().write_2003_oamaddr(value),
            0x2004 => self.ppu_ref.borrow_mut().write_2004_oamdata(value),
            0x2005 => {
                if ppu_ready {
                    self.ppu_ref.borrow_mut().write_2005_ppuscroll(value);
                }
            }
            0x2006 => {
                if ppu_ready {
                    self.ppu_ref.borrow_mut().write_2006_ppuaddr(value);
                }
            }
            0x2007 => self.ppu_ref.borrow_mut().write_2007_ppudata(value, &mut self.mapper),
            0x4014 => { // PPU OAM DMA
                let dma_start = u16::from_le_bytes([0x00, value]);
                let dma_slice = self.mapper.get_cpu_dma_slice(dma_start);

                let mut ppu = self.ppu_ref.borrow_mut();
                ppu.oam_dma(dma_slice);
            },
            0x4016 => self.handle_controller_strobe(value),
            _ => self.mapper.cpu_write(addr, value),
        }
    }

    /// Load a sequence of bytes into cpu memory, starting at addr.
    //pub fn cpu_mem_load(&mut self, addr: u16, data: &[u8]) {
    //    let addr = self.get_cpu_effective_address(addr);
    //    self.cpu_mem.load(addr, data);
    //}

    // Load a sequence of bytes into ppu memory, starting at addr.
    //pub fn ppu_mem_load(&mut self, addr: u16, data: &[u8]) {
    //    let mut ppu = self.ppu_ref.borrow_mut();
    //    ppu.mem_load(addr, data);
    //}

    pub fn ppu_mem_read(&mut self, addr: u16) -> u8 {
        //let addr = self.get_ppu_effective_address(addr);
        self.mapper.ppu_read(addr)
    }

    //pub fn ppu_mem_write(&mut self, addr: u16, value: u8) {
    //    //let addr = self.get_ppu_effective_address(addr);
    //    self.mapper.ppu_write(addr, value)
    //}

    //pub fn load_rom(&mut self, ines: &InesRom) {
    //    self.mapper.load_rom(ines, &mut self.cpu_mem, &mut self.ppu_mem);
    //}

    pub fn handle_controller_strobe(&mut self, value_written: u8) {
        if utils::bit_is_set(0, value_written) {
            self.reload_controller_state = true;
        }
        else {
            //println!("setting controller state (strobe low)");
            self.reload_controller_state = false;

            self.controller1_register = self.controller1_state;
            self.controller1_read_count = 0;

            self.controller2_register = self.controller2_state;
            self.controller2_read_count = 0;

        }
    }

    pub fn set_controller1_state(&mut self, controller_state: u8) {
        self.controller1_state = controller_state;
    }

    pub fn set_controller2_state(&mut self, controller_state: u8) {
        self.controller2_state = controller_state;
    }

    fn read_controller1(&mut self) -> u8 {
        self.controller1_read_count += 1;

        if self.controller1_read_count > 8 {
            0x41 // Return 1 state (0x41 to account for previous bus contents)
        } else {
            //println!("reading controller 1: {:02X}", self.controller1_register);
            let button_value = self.controller1_register & 0x1;
            self.controller1_register = self.controller1_register >> 1;
            0x40 | button_value
        }
    }

    fn read_controller2(&mut self) -> u8 {
        self.controller2_read_count += 1;

        if self.controller2_read_count > 8 {
            0x41 // Return 1 state (0x41 to account for previous bus contents)
        } else {
            let button_value = self.controller2_register & 0x1;
            self.controller2_register >>= 1;
            0x40 | button_value
        }
    }
}