use std::{collections::VecDeque, cell::RefCell, rc::Rc};
pub mod memory;
use memory::Memory;

use crate::ppu::Ppu;

pub enum MemEvent {
    MemRead{ cycle: u64, addr: u16 },
    MemWrite{ cycle: u64, addr: u16, value: u8 },
}

pub trait MemObserver {
    //fn read_happened(&self, addr: u16) -> Option<MemEvent>;
    //fn write_happened(&self, addr: u16, value: u8) -> Option<MemEvent>;
    fn read_happened(&mut self, cycle: u64, addr: u16);
    fn write_happened(&mut self, cycle: u64, addr: u16, value: u8);
}

///
/// Observer for memory read/writes significant to the PPU.
///
#[derive(Default)]
struct PpuMemObserver {
    pub events: VecDeque<MemEvent>,
}

impl MemObserver for PpuMemObserver {
    //fn read_happened(&self, addr: u16) -> Option<MemEvent> {
    //    match addr {
    //        0x2002 => Some(MemEvent::MemRead(addr)),
    //        0x2004 => Some(MemEvent::MemRead(addr)),
    //        0x2007 => Some(MemEvent::MemRead(addr)),
    //        _ => None,
    //    }
    //}
    fn read_happened(&mut self, cycle: u64, addr: u16) {
        if let Some(event) = match addr {
            0x2002 => Some(MemEvent::MemRead{cycle, addr}),
            0x2004 => Some(MemEvent::MemRead{cycle, addr}),
            0x2007 => Some(MemEvent::MemRead{cycle, addr}),
            _ => None,
        }{
            self.events.push_back(event);
        }
    }

    fn write_happened(&mut self, cycle: u64, addr: u16, value: u8) {
        if let Some(event) = match addr {
            0x2000 => Some(MemEvent::MemWrite{cycle, addr, value}),
            0x2001 => Some(MemEvent::MemWrite{cycle, addr, value}),
            0x2003 => Some(MemEvent::MemWrite{cycle, addr, value}),
            0x2004 => Some(MemEvent::MemWrite{cycle, addr, value}),
            0x2005 => Some(MemEvent::MemWrite{cycle, addr, value}),
            0x2006 => Some(MemEvent::MemWrite{cycle, addr, value}),
            0x2007 => Some(MemEvent::MemWrite{cycle, addr, value}),
            0x4014 => Some(MemEvent::MemWrite{cycle, addr, value}),
            _ => None,
        }{
            self.events.push_back(event);
        }
    }
}


///
/// This structure contains state data that needs to be accessed by multiple
/// components plus metadata about events (like read/write) to memory mapped
/// registers. This structure is passed to the CPU, Mapper, PPU, and APU
/// when it's their turn to cycle.
/// 
pub struct MemController {
    cpu_mem: Memory,
    //ppu_mem: Memory,
    //vram: Memory,
    ppu_observer: PpuMemObserver,
    mapper_observer: Box<dyn MemObserver>,
    ppu_ref: Rc<RefCell<Ppu>>,
}

impl MemController {
    pub fn new(mapper_observer: Box<dyn MemObserver>, ppu_ref: Rc<RefCell<Ppu>>) -> Self {
        Self {
            cpu_mem: Memory::new_cpu(),
            //ppu_mem: Memory::new_ppu(),
            //vram: Memory::new_vram(),
            ppu_observer: PpuMemObserver::default(),
            mapper_observer,
            ppu_ref,
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
    pub fn cpu_mem_read(&mut self, cycle: u64, addr: u16) -> u8 {
        let addr = self.get_cpu_effective_address(addr);

        self.ppu_observer.read_happened(cycle, addr);
        self.mapper_observer.read_happened(cycle, addr);

        let read_result = match addr {
            0x2002 => {
                self.ppu_ref.borrow_mut().read_2002_ppustatus()
                //let mut ppu = self.ppu_ref.borrow_mut();
                //ppu.read_2002_ppustatus()
            },
            0x2004 => self.ppu_ref.borrow().read_2004_oamdata(),
            0x2007 => self.ppu_ref.borrow_mut().read_2007_ppudata(),
                 _ => self.cpu_mem.read(addr),
        };

        read_result
    }

    /// Read from CPU memory without notifying observers.
    pub fn _raw_cpu_mem_read(&self, addr: u16) -> u8 {
        let addr = self.get_cpu_effective_address(addr);
        self.cpu_mem.read(addr)
    }

    /// Read a 16-bit value from memory and notify observers.
    pub fn cpu_mem_read_word(&mut self, cycle: u64, addr: u16) -> u16 {
        let addr = self.get_cpu_effective_address(addr);

        self.ppu_observer.read_happened(cycle, addr);
        self.ppu_observer.read_happened(cycle, addr + 1);
        self.mapper_observer.read_happened(cycle, addr);
        self.mapper_observer.read_happened(cycle, addr + 1);

        self.cpu_mem.read_word(addr)
    }
    
    /// Read a 16-bit value from memory without notifying observers.
    pub fn raw_cpu_mem_read_word(&self, addr: u16) -> u16 {
        let addr = self.get_cpu_effective_address(addr);
        self.cpu_mem.read_word(addr)
    }

    /// Write an 8-bit value to memory, properly forwarding writes to PPU register
    /// ports as appropriate.
    pub fn cpu_mem_write(&mut self, cycle: u64, addr: u16, value: u8) {
        let addr = self.get_cpu_effective_address(addr);

        self.ppu_observer.write_happened(cycle, addr, value);
        self.mapper_observer.write_happened(cycle, addr, value);

        // Handle PPU register address writes if necessary.
        match addr {
            0x2000 => self.ppu_ref.borrow_mut().write_2000_ppuctrl(value),
            0x2001 => self.ppu_ref.borrow_mut().write_2001_ppumask(value),
            0x2003 => self.ppu_ref.borrow_mut().write_2003_oamaddr(value),
            0x2004 => self.ppu_ref.borrow_mut().write_2004_oamdata(value),
            0x2005 => self.ppu_ref.borrow_mut().write_2005_ppuscroll(value),
            0x2006 => self.ppu_ref.borrow_mut().write_2006_ppuaddr(value),
            0x2007 => self.ppu_ref.borrow_mut().write_2007_ppudata(value),
            0x4014 => { // PPU OAM DMA
                let dma_start = u16::from_le_bytes([0x00, value]);
                let dma_slice = self.cpu_mem.get_slice(dma_start, 256);

                let mut ppu = self.ppu_ref.borrow_mut();
                ppu.oam_dma(dma_slice);
            },
            _ => self.cpu_mem.write(addr, value),
        }
    }

    /// Load a sequence of bytes into cpu memory, starting at addr.
    pub fn cpu_mem_load(&mut self, addr: u16, data: &[u8]) {
        let addr = self.get_cpu_effective_address(addr);
        self.cpu_mem.load(addr, data);
    }

    /// Load a sequence of bytes into ppu memory, starting at addr.
    pub fn ppu_mem_load(&mut self, addr: u16, data: &[u8]) {
        let mut ppu = self.ppu_ref.borrow_mut();
        ppu.mem_load(addr, data);
    }
}