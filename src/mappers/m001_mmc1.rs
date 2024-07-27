
use super::{Mapper, get_ppu_effective_address};
use crate::ines::{InesRom, PRG_ROM_CHUNK_SIZE};
use crate::mem::Memory;
use crate::ppu::constants::*;
use crate::utils::bit_is_set;

const FOUR_KB: usize = 4096;

#[derive(Debug)]
enum Mirroring {
    Vertical,
    Horizontal,
    OneScreen0,
    OneScreen1,
}

/// Possible statuses returned after pushing a value into the
/// 5-bit shift register.
enum RegisterStatus {
    /// Value pushed to register triggered a clearing of the register.
    /// i.e. Bit 7 of pushed value was 1.
    Cleared,

    /// Five values loaded. Register is ready.
    Ready,

    /// Register is partially filled with less than 5 values.
    Partial,
}

enum PrgRomBankMode {
    /// Switch 32KB banks at 0x8000.
    Bank8000_32KB,

    /// 16KB bank at 0x8000 fixed, bank at 0xC000 switchable.
    Bank8000Fixed,

    /// 16KB bank at 0xC000 fixed, bank at 0x8000 switchable.
    BankC000Fixed,
}

enum ChrRomBankMode {
    /// Switch a single 8KB bank.
    Switch8KB,

    /// Switch two separate 4KB banks.
    Switch4KB,
}

pub struct Mmc1Mapper {
    name: &'static str,
    number: u16,
    cpu_mem: Memory,
    ppu_mem: Memory,
    mirroring: Mirroring,
    prg_rom_banks: Vec<[u8; PRG_ROM_CHUNK_SIZE]>,

    // Store CHR in 4KB bank mode and load N and N+1 banks if in 8KB mode.
    chr_rom_banks: Vec<[u8; FOUR_KB]>,

    /// Internal 5-bit shift register used for gathering data to be loaded
    /// in the mmc1's 5 internal registers.
    shift_register: u8,
    /// Counts up to when five values have been added to the shift register.
    reg_write_count: u8,

    prg_rom_bank_mode: PrgRomBankMode,
    chr_rom_bank_mode: ChrRomBankMode,
}

pub fn new(cpu_mem: Memory, ppu_mem: Memory) -> Mmc1Mapper {
    Mmc1Mapper {
        name: "MMC1",
        number: 1,
        cpu_mem,
        ppu_mem,
        mirroring: Mirroring::Vertical, // Initial value doesn't matter
        prg_rom_banks: Vec::new(),
        chr_rom_banks: Vec::new(),
        shift_register: 0,
        reg_write_count: 0,
        prg_rom_bank_mode: PrgRomBankMode::BankC000Fixed,
        chr_rom_bank_mode: ChrRomBankMode::Switch8KB,
    }
}

impl Mapper for Mmc1Mapper {
    fn name(&self) -> &'static str {
        self.name
    }

    fn number(&self) -> u16 {
        self.number
    }

    fn load_rom(&mut self, ines: &InesRom) {
        self.init_prg_banks(&ines);
        self.init_chr_banks(&ines);

        self.cpu_mem.load(0x8000, &self.prg_rom_banks[0]);
        self.cpu_mem.load(0xC000, &self.prg_rom_banks[ines.header.num_prg_rom_chunks - 1]);

        // Assuming reset vector will load CHR, so not loading it here.
    }
    
    fn cpu_read(&mut self, addr: u16) -> u8 {
        self.cpu_mem.read(addr)
    }
    
    // TODO: update for mmc1
    fn cpu_write(&mut self, addr: u16, value: u8) {

        if addr >= 0x8000 {
            match self.push_shift_reg(value) {
                RegisterStatus::Cleared => self.reset(),
                RegisterStatus::Ready => {
                    match addr {
                        0x8000..=0x9FFF => {
                            self.handle_control_register();
                        },
                        0xA000..=0xBFFF => {
                            self.handle_chr0_register();
                        },
                        0xC000..=0xDFFF => {
                            self.handle_chr1_register();
                        },
                        0xE000..=0xFFFF => {
                            self.handle_prg_register();
                        },
                        _ => panic!("mmc1: invalid address") // Should never happen
                    }

                    self.shift_register = 0;
                }
                RegisterStatus::Partial => (),
            }

        } else {
            self.cpu_mem.write(addr, value);
        }

        //if addr < 0x8000 {
        //    self.cpu_mem.write(addr, value);
        //}
        //else {
        //    let bank = (value & 0x0F) as usize;
        //    //println!("bank switch to {}/{}, addr {}, mem val {}",
        //    //    bank, value, addr, self.cpu_mem.read(addr));
        //    self.cpu_mem.load(0x8000, &self.prg_rom_banks[bank]);
        //}
    }
    
    fn get_cpu_dma_slice(&self, addr: u16) -> &[u8] {
        self.cpu_mem.get_slice(addr, 256)
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        let addr = get_ppu_effective_address(addr);
        self.ppu_mem.read(addr)
    }
    
    // TODO: update for mmc1
    fn ppu_write(&mut self, addr: u16, value: u8) {
        let addr = get_ppu_effective_address(addr);

        match addr {
            NAMETABLE_0..=NAMETABLE_3_END => {
                let mirrored_addresses = self.get_mirrored_addresses(addr);
                self.ppu_mem.write(addr, value);

                for mirrored_addr in mirrored_addresses.iter() {
                    self.ppu_mem.write(*mirrored_addr, value);
                }
            },
            _ => self.ppu_mem.write(addr, value),
        }
    }

}

impl Mmc1Mapper {
    fn init_prg_banks(&mut self, ines: &InesRom) {
        for bank in ines.prg_rom.iter() {
            self.prg_rom_banks.push(*bank);
        }
    }

    fn init_chr_banks(&mut self, ines: &InesRom) {
        println!("Num CHR ROM banks: {}", ines.chr_rom.len());
        if ines.chr_rom.len() > 0 {
            for bank in ines.chr_rom.iter() {
                let mut bank_4k: [u8; FOUR_KB] = [0; FOUR_KB];

                bank_4k.copy_from_slice(&bank[0..4096]);
                self.chr_rom_banks.push(bank_4k);

                // TODO: ensure bank_4k can be reused here
                bank_4k.copy_from_slice(&bank[4096..8192]);
                self.chr_rom_banks.push(bank_4k);
            }
        }
        else {
            let bank_4k: [u8; FOUR_KB] = [0; FOUR_KB];
            self.chr_rom_banks.push(bank_4k);
            let bank_4k: [u8; FOUR_KB] = [0; FOUR_KB];
            self.chr_rom_banks.push(bank_4k);
        }
    }

    fn push_shift_reg(&mut self, value: u8) -> RegisterStatus {
        if bit_is_set(7, value) {
            self.shift_register = 0;
            self.reg_write_count = 0;

            RegisterStatus::Cleared
        } else {
            self.shift_register >>= 1;
            let bit = (value & 0x1) << 4;

            self.shift_register |= bit;

            self.reg_write_count += 1;

            if self.reg_write_count > 5 {
                panic!("mmc1 shift register write should never be greater than 5");
            }
            else if self.reg_write_count == 5 {
                self.reg_write_count = 0;
                RegisterStatus::Ready
            }
            else {
                RegisterStatus::Partial
            }
        }
    }

    fn handle_control_register(&mut self) {
        let mirror_mode = self.shift_register & 0x03;
        let prg_mode = (self.shift_register >> 2) & 0x03;
        let chr_mode = (self.shift_register >> 4) & 0x01;

        self.mirroring = match mirror_mode {
            0 => Mirroring::OneScreen0,
            1 => Mirroring::OneScreen1,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => panic!("mmc1: invalid mirror mode: {mirror_mode}"),
        };

        self.mirror_nametables();

        println!("mirroring mode: {:?}", self.mirroring);

        self.prg_rom_bank_mode = match prg_mode {
            0|1 => PrgRomBankMode::Bank8000_32KB,
            2   => PrgRomBankMode::Bank8000Fixed,
            3   => PrgRomBankMode::BankC000Fixed,
            _ => panic!("mmc1: invalid prg bank mode: {prg_mode}"),
        };

        self.chr_rom_bank_mode = match chr_mode {
            0 => ChrRomBankMode::Switch8KB,
            1 => ChrRomBankMode::Switch4KB,
            _ => panic!("mmc1: invalide chr bank mode: {chr_mode}"),
        };
    }

    fn handle_chr0_register(&mut self) {
        match self.chr_rom_bank_mode {
            ChrRomBankMode::Switch8KB => {
                // We're storing our CHR ROM in banks of 4KB so...
                // Clear bit0 so bank is a multiple of 2. Then, we'll then use bank
                // and bank+1 to get our 8KB bank.
                let bank = (self.shift_register & 0x1E) as usize;
                self.ppu_mem.load(0x0000, &self.chr_rom_banks[bank]);
                self.ppu_mem.load(0x1000, &self.chr_rom_banks[bank+1]);
            },
            ChrRomBankMode::Switch4KB => {
                let bank = self.shift_register as usize;
                self.ppu_mem.load(0x0000, &self.chr_rom_banks[bank]);
            },
        }
    }

    fn handle_chr1_register(&mut self) {
        match self.chr_rom_bank_mode {
            ChrRomBankMode::Switch8KB => (),
            ChrRomBankMode::Switch4KB => {
                let bank = self.shift_register as usize;
                self.ppu_mem.load(0x1000, &self.chr_rom_banks[bank]);
            },
        }
    }

    fn handle_prg_register(&mut self) {
        match self.prg_rom_bank_mode {
            PrgRomBankMode::Bank8000_32KB => {
                // We're storing our PRG ROM in banks of 16KB so...
                // Clear bit0 so bank is a multiple of 2. Then, we'll then use bank
                // and bank+1 to get our 32KB bank.
                let bank = (self.shift_register & 0x1E) as usize;
                self.cpu_mem.load(0x8000, &self.prg_rom_banks[bank]);
                self.cpu_mem.load(0xC000, &self.prg_rom_banks[bank+1]);
            },
            PrgRomBankMode::Bank8000Fixed => {
                let bank = self.shift_register as usize;
                self.cpu_mem.load(0xC000, &self.prg_rom_banks[bank]);
            },
            PrgRomBankMode::BankC000Fixed => {
                let bank = self.shift_register as usize;
                self.cpu_mem.load(0x8000, &self.prg_rom_banks[bank]);
            }
        }
    }

    // TODO: It's late, review this.
    fn reset(&mut self) {
        self.shift_register = 0x0C;
        self.handle_control_register();
        self.shift_register = 0;
        let n = self.prg_rom_banks.len();
        self.cpu_mem.load(0xC000, &self.prg_rom_banks[n-1]);
    }

    // TODO: update for mmc1
    fn get_mirrored_addresses(&self, addr: u16) -> Vec<u16> {
        let mut mirrored_addrs = Vec::new();

        match self.mirroring {
            Mirroring::Horizontal => {
                match addr {
                    NAMETABLE_0..=NAMETABLE_0_END | NAMETABLE_2..=NAMETABLE_2_END => {
                        mirrored_addrs.push(addr + 0x400);
                    },
                    NAMETABLE_1..=NAMETABLE_1_END | NAMETABLE_3..=NAMETABLE_3_END => {
                        mirrored_addrs.push(addr - 0x400);
                    },
                    _ => panic!("address is not within a nametable")
                }
            },
            Mirroring::Vertical => {
                match addr {
                    NAMETABLE_0..=NAMETABLE_0_END | NAMETABLE_1..=NAMETABLE_1_END => {
                        mirrored_addrs.push(addr + 0x800);
                    },
                    NAMETABLE_2..=NAMETABLE_2_END | NAMETABLE_3..=NAMETABLE_3_END => {
                        mirrored_addrs.push(addr - 0x800);
                    },
                    _ => panic!("address is not within a nametable")
                }
            }
            Mirroring::OneScreen0 | Mirroring::OneScreen1 => {
                match addr {
                    NAMETABLE_0..=NAMETABLE_0_END => {
                        mirrored_addrs.push(addr + 0x400); // NT1
                        mirrored_addrs.push(addr + 0x800); // NT2
                        mirrored_addrs.push(addr + 0xC00); // NT3
                    },
                    NAMETABLE_1..=NAMETABLE_1_END => {
                        mirrored_addrs.push(addr - 0x400); // NT0
                        mirrored_addrs.push(addr + 0x400); // NT2
                        mirrored_addrs.push(addr + 0x800); // NT3
                    },
                    NAMETABLE_2..=NAMETABLE_2_END => {
                        mirrored_addrs.push(addr - 0x800); // NT0
                        mirrored_addrs.push(addr - 0x400); // NT1
                        mirrored_addrs.push(addr + 0x400); // NT3
                    },
                    NAMETABLE_3..=NAMETABLE_3_END => {
                        mirrored_addrs.push(addr - 0xC00); // NT0
                        mirrored_addrs.push(addr - 0x800); // NT1
                        mirrored_addrs.push(addr - 0x400); // NT2
                    },
                    _ => panic!("address is not within a nametable")
                }

            },
            // Currently not sure how my implementation for this would
            // be different from OneScreen0
            //Mirroring::OneScreen1 => {
            //},
        }

        return mirrored_addrs;
    }

    fn mirror_nametables(&mut self) {
        match self.mirroring {
            Mirroring::Vertical => {
                let mut nt_slice: [u8; 0x400] = [0; 0x400]; 
                nt_slice.copy_from_slice(self.ppu_mem.get_slice(0x2000, 0x400));
                self.ppu_mem.load(0x2800, &nt_slice);

                nt_slice.copy_from_slice(self.ppu_mem.get_slice(0x2400, 0x400));
                self.ppu_mem.load(0x2C00, &nt_slice);
            }
            Mirroring::Horizontal => {
                let mut nt_slice: [u8; 0x400] = [0; 0x400]; 
                nt_slice.copy_from_slice(self.ppu_mem.get_slice(0x2000, 0x400));
                self.ppu_mem.load(0x2400, &nt_slice);

                nt_slice.copy_from_slice(self.ppu_mem.get_slice(0x2800, 0x400));
                self.ppu_mem.load(0x2C00, &nt_slice);

            }
            Mirroring::OneScreen0 => {
                let mut nt_slice: [u8; 0x400] = [0; 0x400]; 
                nt_slice.copy_from_slice(self.ppu_mem.get_slice(0x2000, 0x400));
                self.ppu_mem.load(0x2400, &nt_slice);
                self.ppu_mem.load(0x2800, &nt_slice);
                self.ppu_mem.load(0x2C00, &nt_slice);
            },
            Mirroring::OneScreen1 => { 
                let mut nt_slice: [u8; 0x400] = [0; 0x400]; 
                nt_slice.copy_from_slice(self.ppu_mem.get_slice(0x2400, 0x400));
                self.ppu_mem.load(0x2000, &nt_slice);
                self.ppu_mem.load(0x2800, &nt_slice);
                self.ppu_mem.load(0x2C00, &nt_slice);
            },
        }
    }

}
