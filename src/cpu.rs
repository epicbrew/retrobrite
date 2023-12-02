use crate::utils;
use crate::mem::MemController;
//use crate::mem::Memory;

mod constants;
use constants::*;

/// Function type for Cpu operations.
type CpuOp = fn(&mut Cpu, mc: &mut MemController);

/// Represents a CPU instruction/opcode.
struct Instruction {
    opcode: u8,
    func: CpuOp,
    addr_mode: AddrMode,
    name: &'static str,
    cycles: u64,
    legal: bool,
}

/// 6502 Addressing modes.
enum AddrMode {
    IMM,
    ABS,
    ZP,
    IMP,
    IND,
    ABX,
    ABY,
    ZPX,
    ZPY,
    IZX,
    IZY,
    REL,
    ACC,
    UNK, // Unknown addressing mode for illegal instructions
}

/// 6502 CPU Registers.
/// Field names violate the snake case convention due to the convention of
/// referring to the 6502 CPU registers as capital letters in all of the 6502
/// documentation.
#[allow(non_snake_case)]
#[derive(Default)]
struct Registers {
    A: u8,  // Accumulator register
    X: u8,  // X index register
    Y: u8,  // Y index register
    PC: u16,// Program Counter
    SP: u8, // Stack pointer
    P: u8,  // Processor status flags
}

///
/// NES 6502 CPU.
/// 
pub struct Cpu {
    /// CPU registers.
    reg: Registers,
    
    /// CPU memory.
    //mem: Memory,

    /// Current cpu instruction being executed.
    opcode: u8,

    /// Address from which the operand (if any) for the current instruction
    /// is read. Or in the case of instructions that store to memory, this
    /// is the address to write to. This value is populated for each instruction
    /// based on the instruction addressing mode. For addressing modes that
    /// do not read from memory, this address is not used or populated.
    operand_address: u16,

    /// Operand value for current instruction. This may be the immediate value (for
    /// IMM addressing mode) or a value read from memory. For operands that are
    /// read from memory, this is the value of the byte at Cpu.operand_address.
    operand_value: u8,

    /// Some instructions incur an additional cycle for crossing a page
    /// boundary when fetching their operand from memory. This only occurs
    /// for specific instructions when using ABX, ABY, or IZY addressing
    /// modes. This field will be set to 1 during operand fetching if a
    /// page penalty would occur. Instructions that can incur this penalty
    /// must add it to Cpu.extra_cycles in their execution function.
    page_penalty: u64,

    /// Extra cycles used by an instruction due to page penalties or branching. 
    extra_cycles: u64,

    /// CPU cycle counter
    cycle_count: u64,

    /// Bookkeeping vec that keeps track of the bytes consumed for each instruction.
    bytes_consumed: Vec<u8>,
}

impl Cpu {

    pub fn new(mc: &MemController) -> Self {
        let mut new_self = Self {
            reg: Registers::default(),
            //mem,
            opcode: 0,
            operand_address: 0,
            operand_value: 0,
            page_penalty: 0,
            extra_cycles: 0,
            cycle_count: 7,
            bytes_consumed: Vec::new(),
        };

        new_self.reg.A = 0;
        new_self.reg.X = 0;
        new_self.reg.Y = 0;
        new_self.reg.PC = mc.raw_cpu_mem_read_word(0xFFFC);
        new_self.reg.SP = 0xFD;
        new_self.reg.P = 0x24;

        new_self
    }

    ///
    /// Reset CPU as if NES reset button was pressed.
    /// Using reset mc documented here:
    ///     https://www.nesdev.org/wiki/CPU_power_up_mc
    /// 
    pub fn _reset(&mut self, mc: &mut MemController) {
        self.reg.PC = mc.raw_cpu_mem_read_word(0xFFFC);
        self.reg.SP -= 3; 
        self.reg.P = 0x34; // Not sure if this is correct
    }

    ///
    /// Execute until to (or slightly beyond) the given cycle number.
    /// Note the CPU may overshoot the given cycle number by the amount of
    /// cycles used by the last instruction.
    /// 
    pub fn cycle_to(&mut self, mc: &mut MemController, cycle: u64) {
        while self.cycle_count < cycle {
            let cycles_used = self.execute(mc);
            self.cycle_count += cycles_used;
        }
    }

    /// Sets the program counter to the given value (for debug/testing).
    pub fn set_program_counter(&mut self, addr: u16) {
        self.reg.PC = addr;
    }

    fn execute(&mut self, mc: &mut MemController) -> u64 {
        // Clear our bookkeeping vector
        self.bytes_consumed.clear();

        // Save the address the next instruction will be read from for logging
        let instruction_address = self.reg.PC;

        // Read next opcode
        self.opcode = self.read_byte(mc);
        let idx = self.opcode as usize;
        let instruction = &Cpu::OP_CODES[idx];

        self.page_penalty = 0;
        self.extra_cycles = 0;
        self.set_operand_address(mc, &instruction.addr_mode);

        self.print_log_line(instruction, instruction_address);

        // Execute 
        (instruction.func)(self, mc);

        let total_cycles = instruction.cycles + self.extra_cycles;

        total_cycles
    }

    fn print_log_line(&self, instruction: &Instruction, address: u16) {
        let mut log_line = format!("{:04X}", address);

        for b in self.bytes_consumed.iter() {
            let b = format!("{:02X}", b);
            log_line.push_str(" ");
            log_line.push_str(&b);
        }

        let pad = 16 - log_line.len() - 2;
        let pad = format!("{:<pad$}", " ");

        log_line.push_str(&pad);
        if instruction.legal {
            log_line.push_str(" ");
        } else {
            log_line.push_str("*");
        }
        log_line.push_str(instruction.name);

        //let register_status = format!("A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:>3},{:>3} CYC:{}",
        //                              self.reg.A, self.reg.X, self.reg.Y, self.reg.P, self.reg.SP,
        //                              10, 100, self.cycle_count);

        let register_status = format!("A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
                                      self.reg.A, self.reg.X, self.reg.Y, self.reg.P, self.reg.SP,
                                      self.cycle_count);

        //let pad = 49 - log_line.len() - 2;
        let pad = 22 - log_line.len() - 2;
        let pad = format!("{:<pad$}", " ");
        log_line.push_str(&pad);

        log_line.push_str(&register_status);

        debug!("{}", log_line);
    }

    ///
    /// This function should be used for all instructions and addressing modes
    /// reads from CPU memory. It approximates the cycle at which the read
    /// occurred by adding the currently executing instruction's cycle time
    /// to the current cycle count (current cycle count is still reflecting
    /// the last instruction's cycles during the execution of a new instruction).
    /// 
    fn do_mem_read(&self, mc: &mut MemController, addr: u16) -> u8 {
        let instruction = &Cpu::OP_CODES[self.opcode as usize];
        let cycle = self.cycle_count + instruction.cycles + self.extra_cycles;

        mc.cpu_mem_read(cycle, addr)
    }

    fn do_mem_read_word(&self, mc: &mut MemController, addr: u16) -> u16 {
        let instruction = &Cpu::OP_CODES[self.opcode as usize];
        let cycle = self.cycle_count + instruction.cycles + self.extra_cycles;

        mc.cpu_mem_read_word(cycle, addr)
    }

    fn do_mem_write(&mut self, mc: &mut MemController, addr: u16, value: u8) {
        let instruction = &Cpu::OP_CODES[self.opcode as usize];
        let cycle = self.cycle_count + instruction.cycles + self.extra_cycles;

        mc.cpu_mem_write(cycle, addr, value);

        if addr == 0x4014 { // PPU OAM DMA port, takes 513 or 514 cycles
            self.extra_cycles += 513;
        }
    }

    fn read_byte(&mut self, mc: &mut MemController) -> u8 {
        let next_byte = self.do_mem_read(mc, self.reg.PC);
        self.bytes_consumed.push(next_byte);
        self.reg.PC += 1;
        next_byte
    }

    fn read_word(&mut self, mc: &mut MemController) -> u16 {
        let lsb = self.read_byte(mc) as u16;
        let msb = self.read_byte(mc) as u16;

        (msb << 8) | lsb
    }

    fn addr_mode_acc(&mut self) {
        self.operand_value = self.reg.A;
    }

    fn addr_mode_imm(&mut self, mc: &mut MemController) {
        self.operand_value = self.read_byte(mc);
    }

    fn addr_mode_abs(&mut self, mc: &mut MemController) {
        self.operand_address = self.read_word(mc);
    }

    fn addr_mode_abx(&mut self, mc: &mut MemController) {
        let base_addres = self.read_word(mc);
        self.operand_address = base_addres.wrapping_add(self.reg.X as u16);
        let add_cycles = if utils::same_page(base_addres, self.operand_address) { 0 } else { 1 };
        self.page_penalty = add_cycles;
    }

    fn addr_mode_aby(&mut self, mc: &mut MemController) {
        let base_addres = self.read_word(mc);
        self.operand_address = base_addres.wrapping_add(self.reg.Y as u16);
        let add_cycles = if utils::same_page(base_addres, self.operand_address) { 0 } else { 1 };
        self.page_penalty = add_cycles;
    }

    fn addr_mode_ind(&mut self, mc: &mut MemController) {
        //
        // The following mimics a bug in the 6502 as described by the obelisk
        // 6502 reference as: "An original 6502 has does not correctly fetch the target 
        // address if the indirect vector falls on a page boundary (e.g. $xxFF where xx
        // is any value from $00 to $FF). In this case fetches the LSB from $xxFF as
        // expected but takes the MSB from $xx00."
        //
        let lsb_addr = self.read_word(mc);
        let msb_addr = if lsb_addr & 0x00FF == 0x00FF {
            lsb_addr & 0xFF00
        } else {
            lsb_addr + 1
        };

        let target_lsb = self.do_mem_read(mc, lsb_addr);
        let target_msb = self.do_mem_read(mc, msb_addr);

        self.operand_address = u16::from_le_bytes([target_lsb, target_msb]);
    }

    fn addr_mode_izx(&mut self, mc: &mut MemController) {
        let zp_addr = self.read_byte(mc).wrapping_add(self.reg.X);

        if zp_addr == 0xFF {
            // Need to wrap around the zero page boundary to read memory address
            // from 0x00FF and 0x0000 
            let lsb = self.do_mem_read(mc, zp_addr as u16);
            let msb = self.do_mem_read(mc, 0x00);
            self.operand_address = u16::from_le_bytes([lsb, msb]);
        } else {
            self.operand_address = self.do_mem_read_word(mc, zp_addr as u16);
        }
        //self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_izy(&mut self, mc: &mut MemController) {
        let zp_addr = self.read_byte(mc);
        let base_addr = if zp_addr == 0xFF {
            // Need to wrap around the zero page boundary to read memory address
            // from 0x00FF and 0x0000 
            let lsb = self.do_mem_read(mc, zp_addr as u16);
            let msb = self.do_mem_read(mc, 0x00);
            u16::from_le_bytes([lsb, msb])
        }
        else {
            self.do_mem_read_word(mc, zp_addr as u16)
        };
        self.operand_address = base_addr.wrapping_add(self.reg.Y as u16);
        let add_cycles = if utils::same_page(base_addr, self.operand_address) { 0 } else { 1 };
        self.page_penalty = add_cycles;
        //self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_zp(&mut self, mc: &mut MemController) {
        self.operand_address = self.read_byte(mc) as u16;
        //self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_zpx(&mut self, mc: &mut MemController) {
        let zp_addr = self.read_byte(mc).wrapping_add(self.reg.X);
        self.operand_address = zp_addr as u16;
        //self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_zpy(&mut self, mc: &mut MemController) {
        let zp_addr = self.read_byte(mc).wrapping_add(self.reg.Y);
        self.operand_address = zp_addr as u16;
        //self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_rel(&mut self, mc: &mut MemController) {
        self.operand_value = self.read_byte(mc);
    }

    fn set_operand_address(&mut self, mc: &mut MemController, addr_mode: &AddrMode) {
        match addr_mode {
            AddrMode::IMM => self.addr_mode_imm(mc),
            AddrMode::ABS => self.addr_mode_abs(mc),
            AddrMode::ZP  => self.addr_mode_zp(mc),
            AddrMode::IND => self.addr_mode_ind(mc),
            AddrMode::ABX => self.addr_mode_abx(mc),
            AddrMode::ABY => self.addr_mode_aby(mc),
            AddrMode::ZPX => self.addr_mode_zpx(mc),
            AddrMode::ZPY => self.addr_mode_zpy(mc),
            AddrMode::IZX => self.addr_mode_izx(mc),
            AddrMode::IZY => self.addr_mode_izy(mc),
            AddrMode::REL => self.addr_mode_rel(mc),
            AddrMode::ACC => self.addr_mode_acc(),
            AddrMode::IMP => {},
            AddrMode::UNK => {},
        }
    }

    ///
    /// Fetches operands that need to be read from memory using the operand
    /// address set during the addr_mode function.
    /// 
    fn fetch_operand(&mut self, mc: &mut MemController) {
        let instruction = &Cpu::OP_CODES[self.opcode as usize];

        match instruction.addr_mode {
            AddrMode::IMM => {}, // read_byte() called during addr_mode_imm()
            AddrMode::ABS => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::ZP  => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::IMP => {},
            AddrMode::IND => {},
            AddrMode::ABX => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::ABY => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::ZPX => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::ZPY => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::IZX => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::IZY => self.operand_value = self.do_mem_read(mc, self.operand_address),
            AddrMode::REL => {}, // read_byte() called during addr_mode_rel()
            AddrMode::ACC => {},
            AddrMode::UNK => {},
        }
    }

    fn update_processor_status_n_flag(&mut self, input: u8) {
        utils::set_bit_from(PS_N_BIT, input, &mut self.reg.P);
    }

    fn update_processor_status_z_flag(&mut self, input: u8) {
        match input {
            0 => utils::set_bit(PS_Z_BIT, &mut self.reg.P),
            _ => utils::clear_bit(PS_Z_BIT, &mut self.reg.P),
        };
    }

    fn update_processor_status_nz_flags(&mut self, input: u8) {
        self.update_processor_status_n_flag(input);
        self.update_processor_status_z_flag(input);
    }

    fn set_processor_status_c_flag(&mut self) {
        utils::set_bit(PS_C_BIT, &mut self.reg.P);
    }

    fn clear_processor_status_c_flag(&mut self) {
        utils::clear_bit(PS_C_BIT, &mut self.reg.P);
    }

    fn set_processor_status_v_flag(&mut self) {
        utils::set_bit(PS_V_BIT, &mut self.reg.P);
    }

    fn clear_processor_status_v_flag(&mut self) {
        utils::clear_bit(PS_V_BIT, &mut self.reg.P);
    }

    ///
    /// This function should be called from CPU instruction functions that might
    /// incur a +1 cycle penalty for crossing a page boundary. Do not call this
    /// function from other instruction functions that are not affected by page
    /// boundary crossings.
    /// 
    fn apply_page_penalty(&mut self) {
        self.extra_cycles += self.page_penalty;
    }

    /// Returns the result of adding a signed relative offset to PC.
    /// All branch instructions need this calculation.
    fn get_branch_destination(&self) -> u16 {
        let relative_offset: i8 = self.operand_value as i8;
        self.reg.PC.wrapping_add(relative_offset as u16)
    }

    /// Branch if the specified flag bit (0..7) of P is set.
    fn branch_if_set(&mut self, flag_bit: u8) {
        if utils::bit_is_set(flag_bit, self.reg.P) {
            self.extra_cycles += 1; // +1 cycle when branch succeeds
            let dest = self.get_branch_destination();

            if !utils::same_page(dest, self.reg.PC) {
                self.extra_cycles += 1;
            }

            self.reg.PC = dest;
        }
    }

    /// Branch if the specified flag bit (0..7) of P is clear.
    fn branch_if_clear(&mut self, flag_bit: u8) {
        if !utils::bit_is_set(flag_bit, self.reg.P) {
            self.extra_cycles += 1; // +1 cycle when branch succeeds
            let dest = self.get_branch_destination();

            if !utils::same_page(dest, self.reg.PC) {
                self.extra_cycles += 1;
            }

            self.reg.PC = dest;
        }
    }

    /// Push a value onto the stack.
    fn stack_push(&mut self, mc: &mut MemController, value: u8) {
        // 6502 implements an "empty" stack, so SP points to next empty slot
        let push_addr: u16 = 0x0100 + self.reg.SP as u16;
        self.do_mem_write(mc, push_addr, value);
        self.reg.SP = self.reg.SP.wrapping_sub(1);
    }

    /// Pull a value from the stack.
    fn stack_pull(&mut self, mc: &mut MemController) -> u8 {
        // increment SP to point to the top value of the stack
        self.reg.SP = self.reg.SP.wrapping_add(1);
        let pull_addr: u16 = 0x0100 + self.reg.SP as u16;

        self.do_mem_read(mc, pull_addr)
    }

    /// Push u16 value onto stack.
    fn stack_push_word(&mut self, mc: &mut MemController, value: u16) {
        let le_bytes = value.to_le_bytes();
        self.stack_push(mc, le_bytes[1]);
        self.stack_push(mc, le_bytes[0]);
    }

    fn stack_pull_word(&mut self, mc: &mut MemController) -> u16 {
        let lsb = self.stack_pull(mc);
        let msb = self.stack_pull(mc);
        u16::from_le_bytes([lsb, msb])
    }

    /// Helper function that sets P from stack while disregarding bits 4 and 5.
    fn set_p_from_stack(&mut self, mc: &mut MemController) {
        let stack_value = self.stack_pull(mc);
        utils::set_bit_from(0, stack_value, &mut self.reg.P);
        utils::set_bit_from(1, stack_value, &mut self.reg.P);
        utils::set_bit_from(2, stack_value, &mut self.reg.P);
        utils::set_bit_from(3, stack_value, &mut self.reg.P);
        utils::set_bit_from(6, stack_value, &mut self.reg.P);
        utils::set_bit_from(7, stack_value, &mut self.reg.P);
    }

    //
    // CPU Instructions
    //
    fn and(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.reg.A &= self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn adc(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);

        let carry = if utils::bit_is_set(PS_C_BIT, self.reg.P) { 1u8 } else { 0u8 };

        let (u8_result1, u8_overflow1) = self.reg.A.overflowing_add(self.operand_value);
        let (u8_result2, u8_overflow2) = u8_result1.overflowing_add(carry);

        let reg_a_signed = self.reg.A as i8;
        let (i8_result1, i8_overflow1) = reg_a_signed.overflowing_add(self.operand_value as i8);
        let (         _, i8_overflow2) = i8_result1.overflowing_add(carry as i8);

        if u8_overflow1 || u8_overflow2 {
            self.set_processor_status_c_flag();
        } else {
            self.clear_processor_status_c_flag();
        }

        if i8_overflow1 || i8_overflow2 {
            self.set_processor_status_v_flag();
        } else {
            self.clear_processor_status_v_flag();
        }

        self.reg.A = u8_result2;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn asl(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);

        // Move bit 7 of operand value into carry flag
        if utils::bit_is_set(7, self.operand_value) {
            utils::set_bit(PS_C_BIT, &mut self.reg.P);
        } else {
            utils::clear_bit(PS_C_BIT, &mut self.reg.P);
        }

        // Do the shift
        let result = self.operand_value << 1;
        self.update_processor_status_nz_flags(result);

        let instruction = &Cpu::OP_CODES[self.opcode as usize];

        // Put result in A or memory depending on addressing mode
        match instruction.addr_mode {
            AddrMode::ACC => self.reg.A = result,
            _ => self.do_mem_write(mc, self.operand_address, result),
        };
    }

    fn bcc(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_clear(PS_C_BIT);
    }

    fn bcs(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_set(PS_C_BIT);
    }

    fn beq(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_set(PS_Z_BIT);
    }

    fn bit(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);

        let and_result = self.reg.A & self.operand_value;
        self.update_processor_status_z_flag(and_result);

        utils::set_bit_from(6, self.operand_value, &mut self.reg.P);
        utils::set_bit_from(7, self.operand_value, &mut self.reg.P);
    }

    fn bmi(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_set(PS_N_BIT);
    }

    fn bne(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_clear(PS_Z_BIT);
    }

    fn bpl(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_clear(PS_N_BIT);
    }

    fn brk(&mut self, mc: &mut MemController) {
        self.stack_push_word(mc, self.reg.PC);

        let mut p_val = self.reg.P;
        utils::set_bit(4, &mut p_val);
        utils::set_bit(5, &mut p_val);
        self.stack_push(mc, p_val);

        self.reg.PC = self.do_mem_read_word(mc, 0xFFFE);
        utils::set_bit(PS_B_BIT, &mut self.reg.P);
    }

    fn bvc(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_clear(PS_V_BIT);
    }

    fn bvs(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.branch_if_set(PS_V_BIT);
    }

    fn clc(&mut self, _mc: &mut MemController) {
        utils::clear_bit(PS_C_BIT, &mut self.reg.P);
    }

    fn cld(&mut self, _mc: &mut MemController) {
        utils::clear_bit(PS_D_BIT, &mut self.reg.P);
    }

    fn cli(&mut self, _mc: &mut MemController) {
        utils::clear_bit(PS_I_BIT, &mut self.reg.P);
    }

    fn clv(&mut self, _mc: &mut MemController) {
        utils::clear_bit(PS_V_BIT, &mut self.reg.P);
    }

    fn do_comparison(&mut self, register: u8, mem_value: u8) {
        if register >= mem_value {
            utils::set_bit(PS_C_BIT, &mut self.reg.P);
        } else {
            utils::clear_bit(PS_C_BIT, &mut self.reg.P);
        }

        if register == mem_value {
            utils::set_bit(PS_Z_BIT, &mut self.reg.P);
        } else {
            utils::clear_bit(PS_Z_BIT, &mut self.reg.P);
        }

        let result = register.wrapping_sub(mem_value);
        utils::set_bit_from(PS_N_BIT, result, &mut self.reg.P);
    }

    fn cmp(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.do_comparison(self.reg.A, self.operand_value);
        self.apply_page_penalty();
    }

    fn cpx(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.do_comparison(self.reg.X, self.operand_value);
    }

    fn cpy(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.do_comparison(self.reg.Y, self.operand_value);
    }

    fn dcp(&mut self, mc: &mut MemController) {
        let mut value = self.do_mem_read(mc, self.operand_address);
        value = value.wrapping_sub(1);
        self.update_processor_status_nz_flags(value);
        self.do_mem_write(mc, self.operand_address, value);

        self.do_comparison(self.reg.A, value);
    }

    fn dec(&mut self, mc: &mut MemController) {
        let mut value = self.do_mem_read(mc, self.operand_address);
        value = value.wrapping_sub(1);
        self.update_processor_status_nz_flags(value);
        self.do_mem_write(mc, self.operand_address, value);
    }

    fn dex(&mut self, _mc: &mut MemController) {
        self.reg.X = self.reg.X.wrapping_sub(1);
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn dey(&mut self, _mc: &mut MemController) {
        self.reg.Y = self.reg.Y.wrapping_sub(1);
        self.update_processor_status_nz_flags(self.reg.Y);
    }

    fn eor(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.reg.A ^= self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn inc(&mut self, mc: &mut MemController) {
        let mut value = self.do_mem_read(mc, self.operand_address);
        value = value.wrapping_add(1);
        self.update_processor_status_nz_flags(value);
        self.do_mem_write(mc, self.operand_address, value);
    }

    fn inx(&mut self, _mc: &mut MemController) {
        self.reg.X = self.reg.X.wrapping_add(1);
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn iny(&mut self, _mc: &mut MemController) {
        self.reg.Y = self.reg.Y.wrapping_add(1);
        self.update_processor_status_nz_flags(self.reg.Y);
    }

    fn isb(&mut self, mc: &mut MemController) {
        self.inc(mc);
        //self.operand_value = self.mem.read(self.operand_address);
        self.sbc(mc);
        // This instruction does not suffer a page penalty, but sbc()
        // adds it in, so subtract it back off here.
        self.extra_cycles -= self.page_penalty;
    }

    fn jmp(&mut self, _mc: &mut MemController) {
        self.reg.PC = self.operand_address;
    }

    fn jsr(&mut self, mc: &mut MemController) {
        let return_addr = self.reg.PC - 1;
        self.stack_push_word(mc, return_addr);
        self.reg.PC = self.operand_address;
    }

    fn lax(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.reg.A = self.operand_value;
        self.reg.X = self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn lda(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.reg.A = self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn ldx(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.reg.X = self.operand_value;
        self.update_processor_status_nz_flags(self.reg.X);
        self.apply_page_penalty();
    }

    fn ldy(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.reg.Y = self.operand_value;
        self.update_processor_status_nz_flags(self.reg.Y);
        self.apply_page_penalty();
    }

    fn lsr(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);

        // Move bit 0 of operand value into carry flag
        if utils::bit_is_set(0, self.operand_value) {
            utils::set_bit(PS_C_BIT, &mut self.reg.P);
        } else {
            utils::clear_bit(PS_C_BIT, &mut self.reg.P);
        }

        // Do the shift
        let result = self.operand_value >> 1;
        self.update_processor_status_nz_flags(result);

        let instruction = &Cpu::OP_CODES[self.opcode as usize];

        // Put result in A or memory depending on addressing mode
        match instruction.addr_mode {
            AddrMode::ACC => self.reg.A = result,
            _ => self.do_mem_write(mc, self.operand_address, result),
        };
    }

    fn nop(&mut self, _mc: &mut MemController) {
        // Does nothing

        // Here for illegal nop opcode cycle time accuracy
        self.apply_page_penalty();
    }

    fn ora(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);
        self.reg.A |= self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn pha(&mut self, mc: &mut MemController) {
        self.stack_push(mc, self.reg.A);
    }

    fn php(&mut self, mc: &mut MemController) {
        let mut p_val = self.reg.P;
        utils::set_bit(4, &mut p_val);
        utils::set_bit(5, &mut p_val);
        self.stack_push(mc, p_val);
    }

    fn pla(&mut self, mc: &mut MemController) {
        self.reg.A = self.stack_pull(mc);
        self.update_processor_status_nz_flags(self.reg.A);
    }

    fn plp(&mut self, mc: &mut MemController) {
        self.set_p_from_stack(mc);
    }

    fn rla(&mut self, mc: &mut MemController) {
        self.rol(mc);
        //self.operand_value = self.mem.read(self.operand_address);
        self.and(mc);
        // This instruction does not suffer a page penalty, but and()
        // adds it in, so subtract it back off here.
        self.extra_cycles -= self.page_penalty;
    }

    fn rol(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);

        // Get value for bit 0 from current carry flag value
        let bit0 = if utils::bit_is_set(PS_C_BIT, self.reg.P) { 1u8 } else { 0u8 };

        // Move bit 7 of operand value into carry flag
        if utils::bit_is_set(7, self.operand_value) {
            utils::set_bit(PS_C_BIT, &mut self.reg.P);
        } else {
            utils::clear_bit(PS_C_BIT, &mut self.reg.P);
        }

        // Do the shift
        let mut result = self.operand_value << 1;

        // Move old carry value into result's bit 0
        result |= bit0;

        self.update_processor_status_nz_flags(result);

        let instruction = &Cpu::OP_CODES[self.opcode as usize];

        // Put result in A or memory depending on addressing mode
        match instruction.addr_mode {
            AddrMode::ACC => self.reg.A = result,
            _ => self.do_mem_write(mc, self.operand_address, result),
        };
    }

    fn ror(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);

        // Get value for bit 7 from current carry flag value
        let bit7 = if utils::bit_is_set(PS_C_BIT, self.reg.P) { 1u8 << 7 } else { 0u8 };

        // Move bit 0 of operand value into carry flag
        if utils::bit_is_set(0, self.operand_value) {
            utils::set_bit(PS_C_BIT, &mut self.reg.P);
        } else {
            utils::clear_bit(PS_C_BIT, &mut self.reg.P);
        }

        // Do the shift
        let mut result = self.operand_value >> 1;

        // Move old carry value into result's bit 0
        result |= bit7;

        self.update_processor_status_nz_flags(result);

        let instruction = &Cpu::OP_CODES[self.opcode as usize];

        // Put result in A or memory depending on addressing mode
        match instruction.addr_mode {
            AddrMode::ACC => self.reg.A = result,
            _ => self.do_mem_write(mc, self.operand_address, result),
        };
    }

    fn rra(&mut self, mc: &mut MemController) {
        self.ror(mc);
        //self.operand_value = self.mem.read(self.operand_address);
        self.adc(mc);
        // This instruction does not suffer a page penalty, but adc()
        // adds it in, so subtract it back off here.
        self.extra_cycles -= self.page_penalty;
    }

    fn rti(&mut self, mc: &mut MemController) {
        self.set_p_from_stack(mc);
        self.reg.PC = self.stack_pull_word(mc);
    }

    fn rts(&mut self, mc: &mut MemController) {
        self.reg.PC = self.stack_pull_word(mc);
        self.reg.PC += 1;
    }

    fn sax(&mut self, mc: &mut MemController) {
        let result = self.reg.A & self.reg.X;
        self.do_mem_write(mc, self.operand_address, result);
    }

    fn sbc(&mut self, mc: &mut MemController) {
        self.fetch_operand(mc);

        let carry = if utils::bit_is_set(PS_C_BIT, self.reg.P) { 0u8 } else { 1u8 };

        let (u8_result1, u8_overflow1) = self.reg.A.overflowing_sub(self.operand_value);
        let (u8_result2, u8_overflow2) = u8_result1.overflowing_sub(carry);

        let reg_a_signed = self.reg.A as i8;
        let (i8_result1, i8_overflow1) = reg_a_signed.overflowing_sub(self.operand_value as i8);
        let (         _, i8_overflow2) = i8_result1.overflowing_sub(carry as i8);

        if u8_overflow1 || u8_overflow2 {
            self.clear_processor_status_c_flag();
        } else {
            self.set_processor_status_c_flag();
        }

        if i8_overflow1 || i8_overflow2 {
            self.set_processor_status_v_flag();
        } else {
            self.clear_processor_status_v_flag();
        }

        self.reg.A = u8_result2;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn sec(&mut self, _mc: &mut MemController) {
        utils::set_bit(PS_C_BIT, &mut self.reg.P);
    }

    fn sed(&mut self, _mc: &mut MemController) {
        utils::set_bit(PS_D_BIT, &mut self.reg.P);
    }

    fn sei(&mut self, _mc: &mut MemController) {
        utils::set_bit(PS_I_BIT, &mut self.reg.P);
    }

    fn slo(&mut self, mc: &mut MemController) {
        self.asl(mc);
        //self.operand_value = self.mem.read(self.operand_address);
        self.ora(mc);
        // This instruction does not suffer a page penalty, but ora()
        // adds it in, so subtract it back off here.
        self.extra_cycles -= self.page_penalty;
    }

    fn sre(&mut self, mc: &mut MemController) {
        self.lsr(mc);
        //self.operand_value = self.mem.read(self.operand_address);
        self.eor(mc);
        // This instruction does not suffer a page penalty, but eor()
        // adds it in, so subtract it back off here.
        self.extra_cycles -= self.page_penalty;
    }

    fn sta(&mut self, mc: &mut MemController) {
        self.do_mem_write(mc, self.operand_address, self.reg.A);
    }

    fn stx(&mut self, mc: &mut MemController) {
        self.do_mem_write(mc, self.operand_address, self.reg.X);
    }

    fn sty(&mut self, mc: &mut MemController) {
        self.do_mem_write(mc, self.operand_address, self.reg.Y);
    }

    fn tax(&mut self, _mc: &mut MemController) {
        self.reg.X = self.reg.A;
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn tay(&mut self, _mc: &mut MemController) {
        self.reg.Y = self.reg.A;
        self.update_processor_status_nz_flags(self.reg.Y);
    }

    fn tsx(&mut self, _mc: &mut MemController) {
        self.reg.X = self.reg.SP;
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn txa(&mut self, _mc: &mut MemController) {
        self.reg.A = self.reg.X;
        self.update_processor_status_nz_flags(self.reg.A);
    }

    fn txs(&mut self, _mc: &mut MemController) {
        self.reg.SP = self.reg.X;
    }

    fn tya(&mut self, _mc: &mut MemController) {
        self.reg.A = self.reg.Y;
        self.update_processor_status_nz_flags(self.reg.A);
    }

    fn oops(&mut self, _mc: &mut MemController) {
        let idx = self.opcode as usize;
        let instr = &Cpu::OP_CODES[idx];
        error!("unsupported instruction: {:#04x}  ({})", instr.opcode, instr.name);
        std::process::exit(1);
    }

    //
    // Jump table of all possible instructions (including 6502 undocumented instructions).
    //
    const OP_CODES: [Instruction; 256] = [
        Instruction {opcode: 0x00, func: Cpu::brk,  addr_mode: AddrMode::IMP, name: "BRK", cycles: 7, legal: true },
        Instruction {opcode: 0x01, func: Cpu::ora,  addr_mode: AddrMode::IZX, name: "ORA", cycles: 6, legal: true },
        Instruction {opcode: 0x02, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x03, func: Cpu::slo,  addr_mode: AddrMode::IZX, name: "SLO", cycles: 8, legal: false },
        Instruction {opcode: 0x04, func: Cpu::nop,  addr_mode: AddrMode::ZP,  name: "NOP", cycles: 3, legal: false },
        Instruction {opcode: 0x05, func: Cpu::ora,  addr_mode: AddrMode::ZP,  name: "ORA", cycles: 3, legal: true },
        Instruction {opcode: 0x06, func: Cpu::asl,  addr_mode: AddrMode::ZP,  name: "ASL", cycles: 5, legal: true },
        Instruction {opcode: 0x07, func: Cpu::slo,  addr_mode: AddrMode::ZP,  name: "SLO", cycles: 5, legal: false },
        Instruction {opcode: 0x08, func: Cpu::php,  addr_mode: AddrMode::IMP, name: "PHP", cycles: 3, legal: true },
        Instruction {opcode: 0x09, func: Cpu::ora,  addr_mode: AddrMode::IMM, name: "ORA", cycles: 2, legal: true },
        Instruction {opcode: 0x0A, func: Cpu::asl,  addr_mode: AddrMode::ACC, name: "ASL", cycles: 2, legal: true },
        Instruction {opcode: 0x0B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ANC", cycles: 2, legal: false },
        Instruction {opcode: 0x0C, func: Cpu::nop,  addr_mode: AddrMode::ABS, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x0D, func: Cpu::ora,  addr_mode: AddrMode::ABS, name: "ORA", cycles: 4, legal: true },
        Instruction {opcode: 0x0E, func: Cpu::asl,  addr_mode: AddrMode::ABS, name: "ASL", cycles: 6, legal: true },
        Instruction {opcode: 0x0F, func: Cpu::slo,  addr_mode: AddrMode::ABS, name: "SLO", cycles: 6, legal: false },

        Instruction {opcode: 0x10, func: Cpu::bpl,  addr_mode: AddrMode::REL, name: "BPL", cycles: 2, legal: true },
        Instruction {opcode: 0x11, func: Cpu::ora,  addr_mode: AddrMode::IZY, name: "ORA", cycles: 5, legal: true },
        Instruction {opcode: 0x12, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x13, func: Cpu::slo,  addr_mode: AddrMode::IZY, name: "SLO", cycles: 8, legal: false },
        Instruction {opcode: 0x14, func: Cpu::nop,  addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x15, func: Cpu::ora,  addr_mode: AddrMode::ZPX, name: "ORA", cycles: 4, legal: true },
        Instruction {opcode: 0x16, func: Cpu::asl,  addr_mode: AddrMode::ZPX, name: "ASL", cycles: 6, legal: true },
        Instruction {opcode: 0x17, func: Cpu::slo,  addr_mode: AddrMode::ZPX, name: "SLO", cycles: 6, legal: false },
        Instruction {opcode: 0x18, func: Cpu::clc,  addr_mode: AddrMode::IMP, name: "CLC", cycles: 2, legal: true },
        Instruction {opcode: 0x19, func: Cpu::ora,  addr_mode: AddrMode::ABY, name: "ORA", cycles: 4, legal: true },
        Instruction {opcode: 0x1A, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0x1B, func: Cpu::slo,  addr_mode: AddrMode::ABY, name: "SLO", cycles: 7, legal: false },
        Instruction {opcode: 0x1C, func: Cpu::nop,  addr_mode: AddrMode::ABX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x1D, func: Cpu::ora,  addr_mode: AddrMode::ABX, name: "ORA", cycles: 4, legal: true },
        Instruction {opcode: 0x1E, func: Cpu::asl,  addr_mode: AddrMode::ABX, name: "ASL", cycles: 7, legal: true },
        Instruction {opcode: 0x1F, func: Cpu::slo,  addr_mode: AddrMode::ABX, name: "SLO", cycles: 7, legal: false },

        Instruction {opcode: 0x20, func: Cpu::jsr,  addr_mode: AddrMode::ABS, name: "JSR", cycles: 6, legal: true },
        Instruction {opcode: 0x21, func: Cpu::and,  addr_mode: AddrMode::IZX, name: "AND", cycles: 6, legal: true },
        Instruction {opcode: 0x22, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x23, func: Cpu::rla,  addr_mode: AddrMode::IZX, name: "RLA", cycles: 8, legal: false },
        Instruction {opcode: 0x24, func: Cpu::bit,  addr_mode: AddrMode::ZP,  name: "BIT", cycles: 3, legal: true },
        Instruction {opcode: 0x25, func: Cpu::and,  addr_mode: AddrMode::ZP,  name: "AND", cycles: 3, legal: true },
        Instruction {opcode: 0x26, func: Cpu::rol,  addr_mode: AddrMode::ZP,  name: "ROL", cycles: 5, legal: true },
        Instruction {opcode: 0x27, func: Cpu::rla,  addr_mode: AddrMode::ZP,  name: "RLA", cycles: 5, legal: false },
        Instruction {opcode: 0x28, func: Cpu::plp , addr_mode: AddrMode::IMP, name: "PLP", cycles: 4, legal: true },
        Instruction {opcode: 0x29, func: Cpu::and,  addr_mode: AddrMode::IMM, name: "AND", cycles: 2, legal: true },
        Instruction {opcode: 0x2A, func: Cpu::rol,  addr_mode: AddrMode::ACC, name: "ROL", cycles: 2, legal: true },
        Instruction {opcode: 0x2B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ANC", cycles: 2, legal: false },
        Instruction {opcode: 0x2C, func: Cpu::bit,  addr_mode: AddrMode::ABS, name: "BIT", cycles: 4, legal: true },
        Instruction {opcode: 0x2D, func: Cpu::and,  addr_mode: AddrMode::ABS, name: "AND", cycles: 4, legal: true },
        Instruction {opcode: 0x2E, func: Cpu::rol,  addr_mode: AddrMode::ABS, name: "ROL", cycles: 6, legal: true },
        Instruction {opcode: 0x2F, func: Cpu::rla,  addr_mode: AddrMode::ABS, name: "RLA", cycles: 6, legal: false },

        Instruction {opcode: 0x30, func: Cpu::bmi,  addr_mode: AddrMode::REL, name: "BMI", cycles: 2, legal: true },
        Instruction {opcode: 0x31, func: Cpu::and,  addr_mode: AddrMode::IZY, name: "AND", cycles: 5, legal: true },
        Instruction {opcode: 0x32, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x33, func: Cpu::rla,  addr_mode: AddrMode::IZY, name: "RLA", cycles: 8, legal: false },
        Instruction {opcode: 0x34, func: Cpu::nop,  addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x35, func: Cpu::and,  addr_mode: AddrMode::ZPX, name: "AND", cycles: 4, legal: true },
        Instruction {opcode: 0x36, func: Cpu::rol,  addr_mode: AddrMode::ZPX, name: "ROL", cycles: 6, legal: true },
        Instruction {opcode: 0x37, func: Cpu::rla,  addr_mode: AddrMode::ZPX, name: "RLA", cycles: 6, legal: false },
        Instruction {opcode: 0x38, func: Cpu::sec,  addr_mode: AddrMode::IMP, name: "SEC", cycles: 2, legal: true },
        Instruction {opcode: 0x39, func: Cpu::and,  addr_mode: AddrMode::ABY, name: "AND", cycles: 4, legal: true },
        Instruction {opcode: 0x3A, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0x3B, func: Cpu::rla,  addr_mode: AddrMode::ABY, name: "RLA", cycles: 7, legal: false },
        Instruction {opcode: 0x3C, func: Cpu::nop,  addr_mode: AddrMode::ABX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x3D, func: Cpu::and,  addr_mode: AddrMode::ABX, name: "AND", cycles: 4, legal: true },
        Instruction {opcode: 0x3E, func: Cpu::rol,  addr_mode: AddrMode::ABX, name: "ROL", cycles: 7, legal: true },
        Instruction {opcode: 0x3F, func: Cpu::rla,  addr_mode: AddrMode::ABX, name: "RLA", cycles: 7, legal: false },

        Instruction {opcode: 0x40, func: Cpu::rti,  addr_mode: AddrMode::IMP, name: "RTI", cycles: 6, legal: true },
        Instruction {opcode: 0x41, func: Cpu::eor,  addr_mode: AddrMode::IZX, name: "EOR", cycles: 6, legal: true },
        Instruction {opcode: 0x42, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x43, func: Cpu::sre,  addr_mode: AddrMode::IZX, name: "SRE", cycles: 8, legal: false },
        Instruction {opcode: 0x44, func: Cpu::nop,  addr_mode: AddrMode::ZP,  name: "NOP", cycles: 3, legal: false },
        Instruction {opcode: 0x45, func: Cpu::eor,  addr_mode: AddrMode::ZP,  name: "EOR", cycles: 3, legal: true },
        Instruction {opcode: 0x46, func: Cpu::lsr,  addr_mode: AddrMode::ZP,  name: "LSR", cycles: 5, legal: true },
        Instruction {opcode: 0x47, func: Cpu::sre,  addr_mode: AddrMode::ZP,  name: "SRE", cycles: 5, legal: false },
        Instruction {opcode: 0x48, func: Cpu::pha,  addr_mode: AddrMode::IMP, name: "PHA", cycles: 3, legal: true },
        Instruction {opcode: 0x49, func: Cpu::eor,  addr_mode: AddrMode::IMM, name: "EOR", cycles: 2, legal: true },
        Instruction {opcode: 0x4A, func: Cpu::lsr,  addr_mode: AddrMode::ACC, name: "LSR", cycles: 2, legal: true },
        Instruction {opcode: 0x4B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ALR", cycles: 2, legal: false },
        Instruction {opcode: 0x4C, func: Cpu::jmp,  addr_mode: AddrMode::ABS, name: "JMP", cycles: 3, legal: true },
        Instruction {opcode: 0x4D, func: Cpu::eor,  addr_mode: AddrMode::ABS, name: "EOR", cycles: 4, legal: true },
        Instruction {opcode: 0x4E, func: Cpu::lsr,  addr_mode: AddrMode::ABS, name: "LSR", cycles: 6, legal: true },
        Instruction {opcode: 0x4F, func: Cpu::sre,  addr_mode: AddrMode::ABS, name: "SRE", cycles: 6, legal: false },

        Instruction {opcode: 0x50, func: Cpu::bvc,  addr_mode: AddrMode::REL, name: "BVC", cycles: 2, legal: true },
        Instruction {opcode: 0x51, func: Cpu::eor,  addr_mode: AddrMode::IZY, name: "EOR", cycles: 5, legal: true },
        Instruction {opcode: 0x52, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x53, func: Cpu::sre,  addr_mode: AddrMode::IZY, name: "SRE", cycles: 8, legal: false },
        Instruction {opcode: 0x54, func: Cpu::nop,  addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x55, func: Cpu::eor,  addr_mode: AddrMode::ZPX, name: "EOR", cycles: 4, legal: true },
        Instruction {opcode: 0x56, func: Cpu::lsr,  addr_mode: AddrMode::ZPX, name: "LSR", cycles: 6, legal: true },
        Instruction {opcode: 0x57, func: Cpu::sre,  addr_mode: AddrMode::ZPX, name: "SRE", cycles: 6, legal: false },
        Instruction {opcode: 0x58, func: Cpu::cli,  addr_mode: AddrMode::IMP, name: "CLI", cycles: 2, legal: true },
        Instruction {opcode: 0x59, func: Cpu::eor,  addr_mode: AddrMode::ABY, name: "EOR", cycles: 4, legal: true },
        Instruction {opcode: 0x5A, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0x5B, func: Cpu::sre,  addr_mode: AddrMode::ABY, name: "SRE", cycles: 7, legal: false },
        Instruction {opcode: 0x5C, func: Cpu::nop,  addr_mode: AddrMode::ABX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x5D, func: Cpu::eor,  addr_mode: AddrMode::ABX, name: "EOR", cycles: 4, legal: true },
        Instruction {opcode: 0x5E, func: Cpu::lsr,  addr_mode: AddrMode::ABX, name: "LSR", cycles: 7, legal: true },
        Instruction {opcode: 0x5F, func: Cpu::sre,  addr_mode: AddrMode::ABX, name: "SRE", cycles: 7, legal: false },

        Instruction {opcode: 0x60, func: Cpu::rts,  addr_mode: AddrMode::IMP, name: "RTS", cycles: 6, legal: true },
        Instruction {opcode: 0x61, func: Cpu::adc,  addr_mode: AddrMode::IZX, name: "ADC", cycles: 6, legal: true },
        Instruction {opcode: 0x62, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x63, func: Cpu::rra,  addr_mode: AddrMode::IZX, name: "RRA", cycles: 8, legal: false },
        Instruction {opcode: 0x64, func: Cpu::nop,  addr_mode: AddrMode::ZP,  name: "NOP", cycles: 3, legal: false },
        Instruction {opcode: 0x65, func: Cpu::adc,  addr_mode: AddrMode::ZP,  name: "ADC", cycles: 3, legal: true },
        Instruction {opcode: 0x66, func: Cpu::ror,  addr_mode: AddrMode::ZP,  name: "ROR", cycles: 5, legal: true },
        Instruction {opcode: 0x67, func: Cpu::rra,  addr_mode: AddrMode::ZP,  name: "RRA", cycles: 5, legal: false },
        Instruction {opcode: 0x68, func: Cpu::pla,  addr_mode: AddrMode::IMP, name: "PLA", cycles: 4, legal: true },
        Instruction {opcode: 0x69, func: Cpu::adc,  addr_mode: AddrMode::IMM, name: "ADC", cycles: 2, legal: true },
        Instruction {opcode: 0x6A, func: Cpu::ror,  addr_mode: AddrMode::ACC, name: "ROR", cycles: 2, legal: true },
        Instruction {opcode: 0x6B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ARR", cycles: 2, legal: false },
        Instruction {opcode: 0x6C, func: Cpu::jmp,  addr_mode: AddrMode::IND, name: "JMP", cycles: 5, legal: true },
        Instruction {opcode: 0x6D, func: Cpu::adc,  addr_mode: AddrMode::ABS, name: "ADC", cycles: 4, legal: true },
        Instruction {opcode: 0x6E, func: Cpu::ror,  addr_mode: AddrMode::ABS, name: "ROR", cycles: 6, legal: true },
        Instruction {opcode: 0x6F, func: Cpu::rra,  addr_mode: AddrMode::ABS, name: "RRA", cycles: 6, legal: false },

        Instruction {opcode: 0x70, func: Cpu::bvs,  addr_mode: AddrMode::REL, name: "BVS", cycles: 2, legal: true },
        Instruction {opcode: 0x71, func: Cpu::adc,  addr_mode: AddrMode::IZY, name: "ADC", cycles: 5, legal: true },
        Instruction {opcode: 0x72, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x73, func: Cpu::rra,  addr_mode: AddrMode::IZY, name: "RRA", cycles: 8, legal: false },
        Instruction {opcode: 0x74, func: Cpu::nop,  addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x75, func: Cpu::adc,  addr_mode: AddrMode::ZPX, name: "ADC", cycles: 4, legal: true },
        Instruction {opcode: 0x76, func: Cpu::ror,  addr_mode: AddrMode::ZPX, name: "ROR", cycles: 6, legal: true },
        Instruction {opcode: 0x77, func: Cpu::rra,  addr_mode: AddrMode::ZPX, name: "RRA", cycles: 6, legal: false },
        Instruction {opcode: 0x78, func: Cpu::sei,  addr_mode: AddrMode::IMP, name: "SEI", cycles: 2, legal: true },
        Instruction {opcode: 0x79, func: Cpu::adc,  addr_mode: AddrMode::ABY, name: "ADC", cycles: 4, legal: true },
        Instruction {opcode: 0x7A, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0x7B, func: Cpu::rra,  addr_mode: AddrMode::ABY, name: "RRA", cycles: 7, legal: false },
        Instruction {opcode: 0x7C, func: Cpu::nop,  addr_mode: AddrMode::ABX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0x7D, func: Cpu::adc,  addr_mode: AddrMode::ABX, name: "ADC", cycles: 4, legal: true },
        Instruction {opcode: 0x7E, func: Cpu::ror,  addr_mode: AddrMode::ABX, name: "ROR", cycles: 7, legal: true },
        Instruction {opcode: 0x7F, func: Cpu::rra,  addr_mode: AddrMode::ABX, name: "RRA", cycles: 7, legal: false },

        Instruction {opcode: 0x80, func: Cpu::nop,  addr_mode: AddrMode::IMM, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0x81, func: Cpu::sta,  addr_mode: AddrMode::IZX, name: "STA", cycles: 6, legal: true },
        Instruction {opcode: 0x82, func: Cpu::nop,  addr_mode: AddrMode::IMM, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0x83, func: Cpu::sax,  addr_mode: AddrMode::IZX, name: "SAX", cycles: 6, legal: false },
        Instruction {opcode: 0x84, func: Cpu::sty,  addr_mode: AddrMode::ZP,  name: "STY", cycles: 3, legal: true },
        Instruction {opcode: 0x85, func: Cpu::sta,  addr_mode: AddrMode::ZP,  name: "STA", cycles: 3, legal: true },
        Instruction {opcode: 0x86, func: Cpu::stx,  addr_mode: AddrMode::ZP,  name: "STX", cycles: 3, legal: true },
        Instruction {opcode: 0x87, func: Cpu::sax,  addr_mode: AddrMode::ZP,  name: "SAX", cycles: 3, legal: false },
        Instruction {opcode: 0x88, func: Cpu::dey,  addr_mode: AddrMode::IMP, name: "DEY", cycles: 2, legal: true },
        Instruction {opcode: 0x89, func: Cpu::nop,  addr_mode: AddrMode::IMM, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0x8A, func: Cpu::txa,  addr_mode: AddrMode::IMP, name: "TXA", cycles: 2, legal: true },
        Instruction {opcode: 0x8B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "XAA", cycles: 2, legal: false },
        Instruction {opcode: 0x8C, func: Cpu::sty,  addr_mode: AddrMode::ABS, name: "STY", cycles: 4, legal: true },
        Instruction {opcode: 0x8D, func: Cpu::sta,  addr_mode: AddrMode::ABS, name: "STA", cycles: 4, legal: true },
        Instruction {opcode: 0x8E, func: Cpu::stx,  addr_mode: AddrMode::ABS, name: "STX", cycles: 4, legal: true },
        Instruction {opcode: 0x8F, func: Cpu::sax,  addr_mode: AddrMode::ABS, name: "SAX", cycles: 4, legal: false },

        Instruction {opcode: 0x90, func: Cpu::bcc,  addr_mode: AddrMode::REL, name: "BCC", cycles: 2, legal: true },
        Instruction {opcode: 0x91, func: Cpu::sta,  addr_mode: AddrMode::IZY, name: "STA", cycles: 6, legal: true },
        Instruction {opcode: 0x92, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0x93, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "AHX", cycles: 6, legal: false },
        Instruction {opcode: 0x94, func: Cpu::sty,  addr_mode: AddrMode::ZPX, name: "STY", cycles: 4, legal: true },
        Instruction {opcode: 0x95, func: Cpu::sta,  addr_mode: AddrMode::ZPX, name: "STA", cycles: 4, legal: true },
        Instruction {opcode: 0x96, func: Cpu::stx,  addr_mode: AddrMode::ZPY, name: "STX", cycles: 4, legal: true },
        Instruction {opcode: 0x97, func: Cpu::sax,  addr_mode: AddrMode::ZPY, name: "SAX", cycles: 4, legal: false },
        Instruction {opcode: 0x98, func: Cpu::tya,  addr_mode: AddrMode::IMP, name: "TYA", cycles: 2, legal: true },
        Instruction {opcode: 0x99, func: Cpu::sta,  addr_mode: AddrMode::ABY, name: "STA", cycles: 5, legal: true },
        Instruction {opcode: 0x9A, func: Cpu::txs,  addr_mode: AddrMode::IMP, name: "TXS", cycles: 2, legal: true },
        Instruction {opcode: 0x9B, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "TAS", cycles: 5, legal: false },
        Instruction {opcode: 0x9C, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "SHY", cycles: 5, legal: false },
        Instruction {opcode: 0x9D, func: Cpu::sta,  addr_mode: AddrMode::ABX, name: "STA", cycles: 5, legal: true },
        Instruction {opcode: 0x9E, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "SHX", cycles: 5, legal: false },
        Instruction {opcode: 0x9F, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "AHX", cycles: 5, legal: false },

        Instruction {opcode: 0xA0, func: Cpu::ldy,  addr_mode: AddrMode::IMM, name: "LDY", cycles: 2, legal: true },
        Instruction {opcode: 0xA1, func: Cpu::lda,  addr_mode: AddrMode::IZX, name: "LDA", cycles: 6, legal: true },
        Instruction {opcode: 0xA2, func: Cpu::ldx,  addr_mode: AddrMode::IMM, name: "LDX", cycles: 2, legal: true },
        Instruction {opcode: 0xA3, func: Cpu::lax,  addr_mode: AddrMode::IZX, name: "LAX", cycles: 6, legal: false },
        Instruction {opcode: 0xA4, func: Cpu::ldy,  addr_mode: AddrMode::ZP,  name: "LDY", cycles: 3, legal: true },
        Instruction {opcode: 0xA5, func: Cpu::lda,  addr_mode: AddrMode::ZP,  name: "LDA", cycles: 3, legal: true },
        Instruction {opcode: 0xA6, func: Cpu::ldx,  addr_mode: AddrMode::ZP,  name: "LDX", cycles: 3, legal: true },
        Instruction {opcode: 0xA7, func: Cpu::lax,  addr_mode: AddrMode::ZP,  name: "LAX", cycles: 3, legal: false },
        Instruction {opcode: 0xA8, func: Cpu::tay,  addr_mode: AddrMode::IMP, name: "TAY", cycles: 2, legal: true },
        Instruction {opcode: 0xA9, func: Cpu::lda,  addr_mode: AddrMode::IMM, name: "LDA", cycles: 2, legal: true },
        Instruction {opcode: 0xAA, func: Cpu::tax,  addr_mode: AddrMode::IMP, name: "TAX", cycles: 2, legal: true },
        Instruction {opcode: 0xAB, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "LAX", cycles: 2, legal: false },
        Instruction {opcode: 0xAC, func: Cpu::ldy,  addr_mode: AddrMode::ABS, name: "LDY", cycles: 4, legal: true },
        Instruction {opcode: 0xAD, func: Cpu::lda,  addr_mode: AddrMode::ABS, name: "LDA", cycles: 4, legal: true },
        Instruction {opcode: 0xAE, func: Cpu::ldx,  addr_mode: AddrMode::ABS, name: "LDX", cycles: 4, legal: true },
        Instruction {opcode: 0xAF, func: Cpu::lax,  addr_mode: AddrMode::ABS, name: "LAX", cycles: 4, legal: false },

        Instruction {opcode: 0xB0, func: Cpu::bcs,  addr_mode: AddrMode::REL, name: "BCS", cycles: 2, legal: true },
        Instruction {opcode: 0xB1, func: Cpu::lda,  addr_mode: AddrMode::IZY, name: "LDA", cycles: 5, legal: true },
        Instruction {opcode: 0xB2, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0xB3, func: Cpu::lax,  addr_mode: AddrMode::IZY, name: "LAX", cycles: 5, legal: false },
        Instruction {opcode: 0xB4, func: Cpu::ldy,  addr_mode: AddrMode::ZPX, name: "LDY", cycles: 4, legal: true },
        Instruction {opcode: 0xB5, func: Cpu::lda,  addr_mode: AddrMode::ZPX, name: "LDA", cycles: 4, legal: true },
        Instruction {opcode: 0xB6, func: Cpu::ldx,  addr_mode: AddrMode::ZPY, name: "LDX", cycles: 4, legal: true },
        Instruction {opcode: 0xB7, func: Cpu::lax,  addr_mode: AddrMode::ZPY, name: "LAX", cycles: 4, legal: false },
        Instruction {opcode: 0xB8, func: Cpu::clv,  addr_mode: AddrMode::IMP, name: "CLV", cycles: 2, legal: true },
        Instruction {opcode: 0xB9, func: Cpu::lda,  addr_mode: AddrMode::ABY, name: "LDA", cycles: 4, legal: true },
        Instruction {opcode: 0xBA, func: Cpu::tsx,  addr_mode: AddrMode::IMP, name: "TSX", cycles: 2, legal: true },
        Instruction {opcode: 0xBB, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "LAS", cycles: 4, legal: false },
        Instruction {opcode: 0xBC, func: Cpu::ldy,  addr_mode: AddrMode::ABX, name: "LDY", cycles: 4, legal: true },
        Instruction {opcode: 0xBD, func: Cpu::lda,  addr_mode: AddrMode::ABX, name: "LDA", cycles: 4, legal: true },
        Instruction {opcode: 0xBE, func: Cpu::ldx,  addr_mode: AddrMode::ABY, name: "LDX", cycles: 4, legal: true },
        Instruction {opcode: 0xBF, func: Cpu::lax,  addr_mode: AddrMode::ABY, name: "LAX", cycles: 4, legal: false },

        Instruction {opcode: 0xC0, func: Cpu::cpy,  addr_mode: AddrMode::IMM, name: "CPY", cycles: 2, legal: true },
        Instruction {opcode: 0xC1, func: Cpu::cmp,  addr_mode: AddrMode::IZX, name: "CMP", cycles: 6, legal: true },
        Instruction {opcode: 0xC2, func: Cpu::nop,  addr_mode: AddrMode::IMM, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0xC3, func: Cpu::dcp,  addr_mode: AddrMode::IZX, name: "DCP", cycles: 8, legal: false },
        Instruction {opcode: 0xC4, func: Cpu::cpy,  addr_mode: AddrMode::ZP,  name: "CPY", cycles: 3, legal: true },
        Instruction {opcode: 0xC5, func: Cpu::cmp,  addr_mode: AddrMode::ZP,  name: "CMP", cycles: 3, legal: true },
        Instruction {opcode: 0xC6, func: Cpu::dec,  addr_mode: AddrMode::ZP,  name: "DEC", cycles: 5, legal: true },
        Instruction {opcode: 0xC7, func: Cpu::dcp,  addr_mode: AddrMode::ZP,  name: "DCP", cycles: 5, legal: false },
        Instruction {opcode: 0xC8, func: Cpu::iny,  addr_mode: AddrMode::IMP, name: "INY", cycles: 2, legal: true },
        Instruction {opcode: 0xC9, func: Cpu::cmp,  addr_mode: AddrMode::IMM, name: "CMP", cycles: 2, legal: true },
        Instruction {opcode: 0xCA, func: Cpu::dex,  addr_mode: AddrMode::IMP, name: "DEX", cycles: 2, legal: true },
        Instruction {opcode: 0xCB, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "AXS", cycles: 2, legal: false },
        Instruction {opcode: 0xCC, func: Cpu::cpy,  addr_mode: AddrMode::ABS, name: "CPY", cycles: 4, legal: true },
        Instruction {opcode: 0xCD, func: Cpu::cmp,  addr_mode: AddrMode::ABS, name: "CMP", cycles: 4, legal: true },
        Instruction {opcode: 0xCE, func: Cpu::dec,  addr_mode: AddrMode::ABS, name: "DEC", cycles: 6, legal: true },
        Instruction {opcode: 0xCF, func: Cpu::dcp,  addr_mode: AddrMode::ABS, name: "DCP", cycles: 6, legal: false },

        Instruction {opcode: 0xD0, func: Cpu::bne,  addr_mode: AddrMode::REL, name: "BNE", cycles: 2, legal: true },
        Instruction {opcode: 0xD1, func: Cpu::cmp,  addr_mode: AddrMode::IZY, name: "CMP", cycles: 5, legal: true },
        Instruction {opcode: 0xD2, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0xD3, func: Cpu::dcp,  addr_mode: AddrMode::IZY, name: "DCP", cycles: 8, legal: false },
        Instruction {opcode: 0xD4, func: Cpu::nop,  addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0xD5, func: Cpu::cmp,  addr_mode: AddrMode::ZPX, name: "CMP", cycles: 4, legal: true },
        Instruction {opcode: 0xD6, func: Cpu::dec,  addr_mode: AddrMode::ZPX, name: "DEC", cycles: 6, legal: true },
        Instruction {opcode: 0xD7, func: Cpu::dcp,  addr_mode: AddrMode::ZPX, name: "DCP", cycles: 6, legal: false },
        Instruction {opcode: 0xD8, func: Cpu::cld,  addr_mode: AddrMode::IMP, name: "CLD", cycles: 2, legal: true },
        Instruction {opcode: 0xD9, func: Cpu::cmp,  addr_mode: AddrMode::ABY, name: "CMP", cycles: 4, legal: true },
        Instruction {opcode: 0xDA, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0xDB, func: Cpu::dcp,  addr_mode: AddrMode::ABY, name: "DCP", cycles: 7, legal: false },
        Instruction {opcode: 0xDC, func: Cpu::nop,  addr_mode: AddrMode::ABX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0xDD, func: Cpu::cmp,  addr_mode: AddrMode::ABX, name: "CMP", cycles: 4, legal: true },
        Instruction {opcode: 0xDE, func: Cpu::dec,  addr_mode: AddrMode::ABX, name: "DEC", cycles: 7, legal: true },
        Instruction {opcode: 0xDF, func: Cpu::dcp,  addr_mode: AddrMode::ABX, name: "DCP", cycles: 7, legal: false },

        Instruction {opcode: 0xE0, func: Cpu::cpx,  addr_mode: AddrMode::IMM, name: "CPX", cycles: 2, legal: true },
        Instruction {opcode: 0xE1, func: Cpu::sbc,  addr_mode: AddrMode::IZX, name: "SBC", cycles: 6, legal: true },
        Instruction {opcode: 0xE2, func: Cpu::nop,  addr_mode: AddrMode::IMM, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0xE3, func: Cpu::isb,  addr_mode: AddrMode::IZX, name: "ISB", cycles: 8, legal: false },
        Instruction {opcode: 0xE4, func: Cpu::cpx,  addr_mode: AddrMode::ZP,  name: "CPX", cycles: 3, legal: true },
        Instruction {opcode: 0xE5, func: Cpu::sbc,  addr_mode: AddrMode::ZP,  name: "SBC", cycles: 3, legal: true },
        Instruction {opcode: 0xE6, func: Cpu::inc,  addr_mode: AddrMode::ZP,  name: "INC", cycles: 5, legal: true },
        Instruction {opcode: 0xE7, func: Cpu::isb,  addr_mode: AddrMode::ZP,  name: "ISB", cycles: 5, legal: false },
        Instruction {opcode: 0xE8, func: Cpu::inx,  addr_mode: AddrMode::IMP, name: "INX", cycles: 2, legal: true },
        Instruction {opcode: 0xE9, func: Cpu::sbc,  addr_mode: AddrMode::IMM, name: "SBC", cycles: 2, legal: true },
        Instruction {opcode: 0xEA, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2, legal: true },
        Instruction {opcode: 0xEB, func: Cpu::sbc,  addr_mode: AddrMode::IMM, name: "SBC", cycles: 2, legal: false },
        Instruction {opcode: 0xEC, func: Cpu::cpx,  addr_mode: AddrMode::ABS, name: "CPX", cycles: 4, legal: true },
        Instruction {opcode: 0xED, func: Cpu::sbc,  addr_mode: AddrMode::ABS, name: "SBC", cycles: 4, legal: true },
        Instruction {opcode: 0xEE, func: Cpu::inc,  addr_mode: AddrMode::ABS, name: "INC", cycles: 6, legal: true },
        Instruction {opcode: 0xEF, func: Cpu::isb,  addr_mode: AddrMode::ABS, name: "ISB", cycles: 6, legal: false },

        Instruction {opcode: 0xF0, func: Cpu::beq,  addr_mode: AddrMode::REL, name: "BEQ", cycles: 2, legal: true },
        Instruction {opcode: 0xF1, func: Cpu::sbc,  addr_mode: AddrMode::IZY, name: "SBC", cycles: 5, legal: true },
        Instruction {opcode: 0xF2, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0, legal: false },
        Instruction {opcode: 0xF3, func: Cpu::isb,  addr_mode: AddrMode::IZY, name: "ISB", cycles: 8, legal: false },
        Instruction {opcode: 0xF4, func: Cpu::nop,  addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0xF5, func: Cpu::sbc,  addr_mode: AddrMode::ZPX, name: "SBC", cycles: 4, legal: true },
        Instruction {opcode: 0xF6, func: Cpu::inc,  addr_mode: AddrMode::ZPX, name: "INC", cycles: 6, legal: true },
        Instruction {opcode: 0xF7, func: Cpu::isb,  addr_mode: AddrMode::ZPX, name: "ISB", cycles: 6, legal: false },
        Instruction {opcode: 0xF8, func: Cpu::sed,  addr_mode: AddrMode::IMP, name: "SED", cycles: 2, legal: true },
        Instruction {opcode: 0xF9, func: Cpu::sbc,  addr_mode: AddrMode::ABY, name: "SBC", cycles: 4, legal: true },
        Instruction {opcode: 0xFA, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2, legal: false },
        Instruction {opcode: 0xFB, func: Cpu::isb,  addr_mode: AddrMode::ABY, name: "ISB", cycles: 7, legal: false },
        Instruction {opcode: 0xFC, func: Cpu::nop,  addr_mode: AddrMode::ABX, name: "NOP", cycles: 4, legal: false },
        Instruction {opcode: 0xFD, func: Cpu::sbc,  addr_mode: AddrMode::ABX, name: "SBC", cycles: 4, legal: true },
        Instruction {opcode: 0xFE, func: Cpu::inc,  addr_mode: AddrMode::ABX, name: "INC", cycles: 7, legal: true },
        Instruction {opcode: 0xFF, func: Cpu::isb,  addr_mode: AddrMode::ABX, name: "ISB", cycles: 7, legal: false },
    ];
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::ppu::Ppu;

    impl Cpu {
        pub fn default() -> Self {
            Self {
                reg: Registers::default(),
                opcode: 0,
                operand_address: 0,
                operand_value: 0,
                page_penalty: 0,
                extra_cycles: 0,
                cycle_count: 0,
                bytes_consumed: Vec::new(),
            }
        }
    }

    fn get_mem_controller() -> MemController {
        MemController::new(
            Rc::new(RefCell::new(Ppu::new())),
        )
    }

    fn get_mc_with_cpu_mem_ramp() -> MemController {
        let mut mc = get_mem_controller();

        for page in 0..256 {
            for byte in 0..256 {
                mc.cpu_mem_write(byte as u64, page * 256 + byte, byte as u8);
            }
        }

        mc
    }

    #[test]
    fn test_cpu_read_byte() {
        let mut mc = get_mc_with_cpu_mem_ramp();
        let mut cpu = Cpu::new(&mc);

        //let pc = 0x400; // address 1024 in decimal, page 4
        cpu.reg.PC = 0x400; // address 1024 in decimal, page 4 

        for i in 0..256 {
            let val = cpu.read_byte(&mut mc);
            println!("read value {:#04x} from address {:#04x}", val, cpu.reg.PC - 1);
            assert!(val == i as u8);
        }
    }

    #[test]
    fn test_cpu_read_word() {
        let pc = 0x2f0; // page 2

        let mut mc = get_mc_with_cpu_mem_ramp();
        let mut cpu = Cpu::new(&mc);

        cpu.reg.PC = pc;

        let val = cpu.read_word(&mut mc);

        assert!(val == 61936);
    }

    #[test]
    fn test_fetch_operand_imm() {
        let mut mc = get_mc_with_cpu_mem_ramp();
        let mut cpu = Cpu::new(&mc);
        cpu.reg.PC = 0x501;

        cpu.set_operand_address(&mut mc, &AddrMode::IMM);
        assert!(cpu.operand_value == 1);
    }

    #[test]
    fn test_fetch_operand_abs() {
        let mut mc = get_mc_with_cpu_mem_ramp();
        let mut cpu = Cpu::new(&mc);
        cpu.reg.PC = 0xff02;
        cpu.opcode = OPCODE_ORA_ABS;

        println!("0xff02 = {}", mc.cpu_mem_read(0, 0xff02));

        // Should read 0x0302 as the address word from PC
        // The value at 0x0302 should be 2 in the mem ramp.
        cpu.set_operand_address(&mut mc, &AddrMode::ABS);
        cpu.fetch_operand(&mut mc);

        println!("cpu.operane_address = {:02X}", cpu.operand_address);
        println!("0x0302 = {}", mc.cpu_mem_read(0, 0x0302));
        println!("cpu.operane_value = {}", cpu.operand_value);
        assert!(cpu.operand_value == 2);
    }

    #[test]
    fn test_and_with_zero_result() {
        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x0F;
        cpu.operand_value = 0xF0;
        cpu.and(&mut mc);

        assert!(cpu.reg.A == 0);
        assert!(utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_and_with_negative_result() {
        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x81;
        cpu.operand_value = 0xF1;
        cpu.and(&mut mc);

        assert!(cpu.reg.A == 0x81);
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_eor() {
        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x0F;
        cpu.operand_value = 0xFF;
        cpu.eor(&mut mc);

        assert!(cpu.reg.A == 0xF0);
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_ora() {
        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x8F;
        cpu.operand_value = 0x71;
        cpu.ora(&mut mc);

        assert!(cpu.reg.A == 0xFF);
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_adc() {
        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();

        cpu.reg.A = 1;
        cpu.operand_value = 1;
        cpu.adc(&mut mc);
        assert!(cpu.reg.A == 2);
        assert!(!utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_V_BIT, cpu.reg.P));

        cpu.clc(&mut mc);
        cpu.reg.A = 1;
        cpu.operand_value = -1i8 as u8;
        cpu.adc(&mut mc);
        assert!(cpu.reg.A == 0);
        assert!(utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_V_BIT, cpu.reg.P));

        cpu.clc(&mut mc);
        cpu.reg.A = 0x7F;
        cpu.operand_value = 1;
        cpu.adc(&mut mc);
        assert!(cpu.reg.A == 128);
        assert!(!utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_V_BIT, cpu.reg.P));

        cpu.clc(&mut mc);
        cpu.reg.A = 0x80;
        cpu.operand_value = 0xFF;
        cpu.adc(&mut mc);
        assert!(cpu.reg.A == 0x7F);
        assert!(utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_V_BIT, cpu.reg.P));
    }

    #[test]
    fn test_asl() {
        // This test actually executes a small program to test ASL.
        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();

        let test_program: Vec<u8> = vec![
            OPCODE_LDA_IMM, 0x81,
            OPCODE_ASL_ACC,
            OPCODE_STA_ABS, 0xFE, 0x00
        ];

        let needed_cycles = 
            Cpu::OP_CODES[OPCODE_LDA_IMM as usize].cycles +
            Cpu::OP_CODES[OPCODE_ASL_ACC as usize].cycles +
            Cpu::OP_CODES[OPCODE_STA_ABS as usize].cycles;

        let start_addr = 0xC000;

        mc.cpu_mem_load(start_addr, &test_program);
        cpu.reg.PC = start_addr;
        cpu.cycle_to(&mut mc, needed_cycles);

        assert!(mc.cpu_mem_read(0, 0x00FE) == 0x02);
        assert!(utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_cmp() {
        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();

        cpu.reg.A = 5;
        cpu.operand_value = 0;
        cpu.cmp(&mut mc);
        assert!(utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_N_BIT, cpu.reg.P));

        cpu.reg.A = 101;
        cpu.operand_value = 101;
        cpu.cmp(&mut mc);
        assert!(utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_N_BIT, cpu.reg.P));

        cpu.reg.A = 101;
        cpu.operand_value = 201;
        cpu.cmp(&mut mc);
        assert!(!utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_jsr_rts() {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut mc = get_mem_controller();
        let mut cpu = Cpu::default();

        let test_program: Vec<u8> = vec![
            OPCODE_JSR_ABS, 0x32, 0xC0,
            OPCODE_LDA_ABS, 0x00, 0x10
        ];

        let subroutine: Vec<u8> = vec![
            OPCODE_LDA_IMM, 0x47,
            OPCODE_STA_ABS, 0x00, 0x10,
            OPCODE_RTS
        ];

        let needed_cycles = 
            Cpu::OP_CODES[OPCODE_JSR_ABS as usize].cycles +
            Cpu::OP_CODES[OPCODE_LDA_ABS as usize].cycles +
            Cpu::OP_CODES[OPCODE_LDA_IMM as usize].cycles +
            Cpu::OP_CODES[OPCODE_STA_ABS as usize].cycles +
            Cpu::OP_CODES[OPCODE_RTS as usize].cycles;

        let start_addr = 0xC000;
        mc.cpu_mem_load(start_addr, &test_program);
        mc.cpu_mem_load(start_addr + 0x32, &subroutine);

        cpu.reg.PC = start_addr;
        cpu.cycle_to(&mut mc, needed_cycles);

        assert!(cpu.reg.A == 0x47);
    }
}