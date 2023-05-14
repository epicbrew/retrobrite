use crate::utils;
use crate::mem::Memory;

mod constants;
use constants::*;

/// Function type for Cpu operations.
type CpuOp = fn(&mut Cpu);

/// Represents a CPU instruction/opcode.
struct Instruction {
    opcode: u8,
    func: CpuOp,
    addr_mode: AddrMode,
    name: &'static str,
    cycles: u64,
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
    mem: Memory,

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
}

impl Cpu {

    pub fn new(mem: Memory) -> Self {
        let mut new_self = Self {
            reg: Registers::default(),
            mem,
            opcode: 0,
            operand_address: 0,
            operand_value: 0,
            page_penalty: 0,
            extra_cycles: 0,
            cycle_count: 0,
        };

        new_self.reset();
        new_self
    }

    ///
    /// Reset CPU as if NES reset button was pressed.
    /// TODO: reset registers to correct values.
    /// Using powerup state documented here:
    ///     https://www.nesdev.org/wiki/CPU_power_up_state
    pub fn reset(&mut self) {
        self.reg.A = 0;
        self.reg.X = 0;
        self.reg.Y = 0;
        self.reg.PC = self.mem.read_word(0xFFFC);
        self.reg.SP = 0xFD;
        self.reg.P = 0;
    }

    pub fn cycle_to(&mut self, cycle: u64) {
        while self.cycle_count < cycle {
            let cycles_used = self.execute();
            self.cycle_count += cycles_used;
        }
    }

    fn execute(&mut self) -> u64 {
        self.opcode = self.read_byte();
        let idx = self.opcode as usize;
        let instruction = &Cpu::OP_CODES[idx];

        self.page_penalty = 0;
        self.extra_cycles = 0;
        self.fetch_operand(&instruction.addr_mode);

        (instruction.func)(self);

        instruction.cycles + self.extra_cycles
    }

    fn read_byte(&mut self) -> u8 {
        let next_byte = self.mem.read(self.reg.PC);
        self.reg.PC += 1;
        next_byte
    }

    fn read_word(&mut self) -> u16 {
        let lsb = self.read_byte() as u16;
        let msb = self.read_byte() as u16;

        (msb << 8) | lsb
    }

    fn addr_mode_acc(&mut self) {
        self.operand_value = self.reg.A;
    }

    fn addr_mode_imm(&mut self) {
        self.operand_value = self.read_byte();
    }

    fn addr_mode_abs(&mut self) {
        self.operand_address = self.read_word();
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_abx(&mut self) {
        let base_addres = self.read_word();
        self.operand_address = base_addres.wrapping_add(self.reg.X as u16);
        let add_cycles = if utils::same_page(base_addres, self.operand_address) { 0 } else { 1 };
        self.page_penalty = add_cycles;
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_aby(&mut self) {
        let base_addres = self.read_word();
        self.operand_address = base_addres.wrapping_add(self.reg.Y as u16);
        let add_cycles = if utils::same_page(base_addres, self.operand_address) { 0 } else { 1 };
        self.page_penalty = add_cycles;
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_ind(&mut self) {
        self.operand_address = self.read_word();
        self.operand_address = self.mem.read_word(self.operand_address);
    }

    fn addr_mode_izx(&mut self) {
        let zp_addr = self.read_byte().wrapping_add(self.reg.X);
        self.operand_address = self.mem.read_word(zp_addr as u16);
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_izy(&mut self) {
        let zp_addr = self.read_byte();
        let base_addr = self.mem.read_word(zp_addr as u16);
        self.operand_address = base_addr.wrapping_add(self.reg.Y as u16);
        let add_cycles = if utils::same_page(base_addr, self.operand_address) { 0 } else { 1 };
        self.page_penalty = add_cycles;
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_zp(&mut self) {
        self.operand_address = self.read_byte() as u16;
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_zpx(&mut self) {
        let zp_addr = self.read_byte().wrapping_add(self.reg.X);
        self.operand_address = zp_addr as u16;
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_zpy(&mut self) {
        let zp_addr = self.read_byte().wrapping_add(self.reg.Y);
        self.operand_address = zp_addr as u16;
        self.operand_value = self.mem.read(self.operand_address);
    }

    fn addr_mode_rel(&mut self) {
        self.operand_value = self.read_byte();
    }

    fn fetch_operand(&mut self, addr_mode: &AddrMode) {
        match addr_mode {
            AddrMode::IMM => self.addr_mode_imm(),
            AddrMode::ABS => self.addr_mode_abs(),
            AddrMode::ZP  => self.addr_mode_zp(),
            AddrMode::IND => self.addr_mode_ind(),
            AddrMode::ABX => self.addr_mode_abx(),
            AddrMode::ABY => self.addr_mode_aby(),
            AddrMode::ZPX => self.addr_mode_zpx(),
            AddrMode::ZPY => self.addr_mode_zpy(),
            AddrMode::IZX => self.addr_mode_izx(),
            AddrMode::IZY => self.addr_mode_izy(),
            AddrMode::REL => self.addr_mode_rel(),
            AddrMode::ACC => self.addr_mode_acc(),
            AddrMode::IMP => {},
            AddrMode::UNK => {},
        }
    }

    fn update_processor_status_n_flag(&mut self, input: u8) {
        self.reg.P = utils::set_bit_from(PS_N_BIT, input, self.reg.P);
    }

    fn update_processor_status_z_flag(&mut self, input: u8) {
        self.reg.P = match input {
            0 => utils::set_bit(PS_Z_BIT, self.reg.P),
            _ => utils::clear_bit(PS_Z_BIT, self.reg.P),
        };
    }

    fn update_processor_status_nz_flags(&mut self, input: u8) {
        self.update_processor_status_n_flag(input);
        self.update_processor_status_z_flag(input);
    }

    fn set_processor_status_c_flag(&mut self) {
        self.reg.P = utils::set_bit(PS_C_BIT, self.reg.P);
    }

    fn clear_processor_status_c_flag(&mut self) {
        self.reg.P = utils::clear_bit(PS_C_BIT, self.reg.P);
    }

    fn set_processor_status_v_flag(&mut self) {
        self.reg.P = utils::set_bit(PS_V_BIT, self.reg.P);
    }

    fn clear_processor_status_v_flag(&mut self) {
        self.reg.P = utils::clear_bit(PS_V_BIT, self.reg.P);
    }

    fn apply_page_penalty(&mut self) {
        self.extra_cycles = self.page_penalty;
    }

    //
    // CPU Instructions
    //
    fn and(&mut self) {
        self.reg.A &= self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn adc(&mut self) {
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

    fn asl(&mut self) {
        //if utils::bit_is_set(7, self.operand_value) {
        //    utils::set_bit(PS_C_BIT, &mut self.reg.P);
        //} else {
        //    self.reg.P = utils::clear_bit(PS_C_BIT, self.reg.P);
        //}

        //let instruction = &Cpu::OP_CODES[self.opcode as usize];

        //let result = self.operand_value << 1;

        //match instruction.addr_mode {
        //    AddrMode::ACC => 
        //}
        self.oops();
    }

    fn bcc(&mut self) {
        self.oops();
    }

    fn bcs(&mut self) {
        self.oops();
    }

    fn beq(&mut self) {
        self.oops();
    }

    fn bit(&mut self) {
        let and_result = self.reg.A & self.operand_value;
        self.update_processor_status_z_flag(and_result);

        self.reg.P = utils::set_bit_from(6, self.operand_value, self.reg.P);
        self.reg.P = utils::set_bit_from(7, self.operand_value, self.reg.P);
    }

    fn bmi(&mut self) {
        self.oops();
    }

    fn bne(&mut self) {
        self.oops();
    }

    fn bpl(&mut self) {
        self.oops();
    }

    fn brk(&mut self) {
        self.oops();
    }

    fn bvc(&mut self) {
        self.oops();
    }

    fn bvs(&mut self) {
        self.oops();
    }

    fn clc(&mut self) {
        self.reg.P = utils::clear_bit(PS_C_BIT, self.reg.P);
    }

    fn cld(&mut self) {
        self.reg.P = utils::clear_bit(PS_D_BIT, self.reg.P);
    }

    fn cli(&mut self) {
        self.reg.P = utils::clear_bit(PS_I_BIT, self.reg.P);
    }

    fn clv(&mut self) {
        self.reg.P = utils::clear_bit(PS_V_BIT, self.reg.P);
    }

    fn cmp(&mut self) {
        self.oops();
    }

    fn cpx(&mut self) {
        self.oops();
    }

    fn cpy(&mut self) {
        self.oops();
    }

    fn dec(&mut self) {
        let mut value = self.mem.read(self.operand_address);
        value = value.wrapping_sub(1);
        self.update_processor_status_nz_flags(value);
        self.mem.write(self.operand_address, value);
    }

    fn dex(&mut self) {
        self.reg.X = self.reg.X.wrapping_sub(1);
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn dey(&mut self) {
        self.reg.Y = self.reg.Y.wrapping_sub(1);
        self.update_processor_status_nz_flags(self.reg.Y);
    }

    fn eor(&mut self) {
        self.reg.A ^= self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn inc(&mut self) {
        let mut value = self.mem.read(self.operand_address);
        value = value.wrapping_add(1);
        self.update_processor_status_nz_flags(value);
        self.mem.write(self.operand_address, value);
    }

    fn inx(&mut self) {
        self.reg.X = self.reg.X.wrapping_add(1);
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn iny(&mut self) {
        self.reg.Y = self.reg.Y.wrapping_add(1);
        self.update_processor_status_nz_flags(self.reg.Y);
    }

    fn jmp(&mut self) {
        self.oops();
    }

    fn jsr(&mut self) {
        self.oops();
    }

    fn lda(&mut self) {
        self.reg.A = self.operand_value;
        self.apply_page_penalty();
    }

    fn ldx(&mut self) {
        self.reg.X = self.operand_value;
        self.apply_page_penalty();
    }

    fn ldy(&mut self) {
        self.reg.Y = self.operand_value;
        self.apply_page_penalty();
    }

    fn lsr(&mut self) {
        self.oops();
    }

    fn nop(&mut self) {
        self.oops();
    }

    fn ora(&mut self) {
        self.reg.A |= self.operand_value;
        self.update_processor_status_nz_flags(self.reg.A);
        self.apply_page_penalty();
    }

    fn pha(&mut self) {
        self.oops();
    }

    fn php(&mut self) {
        self.oops();
    }

    fn pla(&mut self) {
        self.oops();
    }

    fn plp(&mut self) {
        self.oops();
    }

    fn rol(&mut self) {
        self.oops();
    }

    fn ror(&mut self) {
        self.oops();
    }

    fn rti(&mut self) {
        self.oops();
    }

    fn rts(&mut self) {
        self.oops();
    }

    fn sbc(&mut self) {
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

    fn sec(&mut self) {
        self.reg.P = utils::set_bit(PS_C_BIT, self.reg.P);
    }

    fn sed(&mut self) {
        self.reg.P = utils::set_bit(PS_D_BIT, self.reg.P);
    }

    fn sei(&mut self) {
        self.reg.P = utils::set_bit(PS_I_BIT, self.reg.P);
    }

    fn sta(&mut self) {
        self.mem.write(self.operand_address, self.reg.A);
    }

    fn stx(&mut self) {
        self.mem.write(self.operand_address, self.reg.X);
    }

    fn sty(&mut self) {
        self.mem.write(self.operand_address, self.reg.Y);
    }

    fn tax(&mut self) {
        self.reg.X = self.reg.A;
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn tay(&mut self) {
        self.reg.Y = self.reg.A;
        self.update_processor_status_nz_flags(self.reg.Y);
    }

    fn tsx(&mut self) {
        self.reg.X = self.reg.SP;
        self.update_processor_status_nz_flags(self.reg.X);
    }

    fn txa(&mut self) {
        self.reg.A = self.reg.X;
        self.update_processor_status_nz_flags(self.reg.A);
    }

    fn txs(&mut self) {
        self.reg.SP = self.reg.X;
    }

    fn tya(&mut self) {
        self.reg.A = self.reg.Y;
        self.update_processor_status_nz_flags(self.reg.A);
    }

    fn oops(&mut self) {
        let idx = self.opcode as usize;
        let instr = &Cpu::OP_CODES[idx];
        error!("unsupported instruction: {:#04x}  ({})", instr.opcode, instr.name);
        std::process::exit(1);
    }

    const OP_CODES: [Instruction; 256] = [
        Instruction {opcode: 0x00, func: Cpu::brk,  addr_mode: AddrMode::IMP, name: "BRK", cycles: 7 },
        Instruction {opcode: 0x01, func: Cpu::ora,  addr_mode: AddrMode::IZX, name: "ORA", cycles: 6 },
        Instruction {opcode: 0x02, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x03, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "SLO", cycles: 8 },
        Instruction {opcode: 0x04, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "NOP", cycles: 3 },
        Instruction {opcode: 0x05, func: Cpu::ora,  addr_mode: AddrMode::ZP,  name: "ORA", cycles: 3 },
        Instruction {opcode: 0x06, func: Cpu::asl,  addr_mode: AddrMode::ZP,  name: "ASL", cycles: 5 },
        Instruction {opcode: 0x07, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "SLO", cycles: 5 },
        Instruction {opcode: 0x08, func: Cpu::php,  addr_mode: AddrMode::IMP, name: "PHP", cycles: 3 },
        Instruction {opcode: 0x09, func: Cpu::ora,  addr_mode: AddrMode::IMM, name: "ORA", cycles: 2 },
        Instruction {opcode: 0x0A, func: Cpu::asl,  addr_mode: AddrMode::ACC, name: "ASL", cycles: 2 },
        Instruction {opcode: 0x0B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ANC", cycles: 2 },
        Instruction {opcode: 0x0C, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x0D, func: Cpu::ora,  addr_mode: AddrMode::ABS, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x0E, func: Cpu::asl,  addr_mode: AddrMode::ABS, name: "ASL", cycles: 6 },
        Instruction {opcode: 0x0F, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "SLO", cycles: 6 },

        Instruction {opcode: 0x10, func: Cpu::bpl,  addr_mode: AddrMode::REL, name: "BPL", cycles: 2 },
        Instruction {opcode: 0x11, func: Cpu::ora,  addr_mode: AddrMode::IZY, name: "ORA", cycles: 5 },
        Instruction {opcode: 0x12, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x13, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "SLO", cycles: 8 },
        Instruction {opcode: 0x14, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x15, func: Cpu::ora,  addr_mode: AddrMode::ZPX, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x16, func: Cpu::asl,  addr_mode: AddrMode::ZPX, name: "ASL", cycles: 6 },
        Instruction {opcode: 0x17, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "SLO", cycles: 6 },
        Instruction {opcode: 0x18, func: Cpu::clc,  addr_mode: AddrMode::IMP, name: "CLC", cycles: 2 },
        Instruction {opcode: 0x19, func: Cpu::ora,  addr_mode: AddrMode::ABY, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x1A, func: Cpu::oops, addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x1B, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "SLO", cycles: 7 },
        Instruction {opcode: 0x1C, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x1D, func: Cpu::ora,  addr_mode: AddrMode::ABX, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x1E, func: Cpu::asl,  addr_mode: AddrMode::ABX, name: "ASL", cycles: 7 },
        Instruction {opcode: 0x1F, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "SLO", cycles: 7 },

        Instruction {opcode: 0x20, func: Cpu::jsr,  addr_mode: AddrMode::ABS, name: "JSR", cycles: 6 },
        Instruction {opcode: 0x21, func: Cpu::and,  addr_mode: AddrMode::IZX, name: "AND", cycles: 6 },
        Instruction {opcode: 0x22, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x23, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "RLA", cycles: 8 },
        Instruction {opcode: 0x24, func: Cpu::bit,  addr_mode: AddrMode::ZP,  name: "BIT", cycles: 3 },
        Instruction {opcode: 0x25, func: Cpu::and,  addr_mode: AddrMode::ZP,  name: "AND", cycles: 3 },
        Instruction {opcode: 0x26, func: Cpu::rol,  addr_mode: AddrMode::ZP,  name: "ROL", cycles: 5 },
        Instruction {opcode: 0x27, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "RLA", cycles: 5 },
        Instruction {opcode: 0x28, func: Cpu::plp , addr_mode: AddrMode::IMP, name: "PLP", cycles: 4 },
        Instruction {opcode: 0x29, func: Cpu::and,  addr_mode: AddrMode::IMM, name: "AND", cycles: 2 },
        Instruction {opcode: 0x2A, func: Cpu::rol,  addr_mode: AddrMode::ACC, name: "ROL", cycles: 2 },
        Instruction {opcode: 0x2B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ANC", cycles: 2 },
        Instruction {opcode: 0x2C, func: Cpu::bit,  addr_mode: AddrMode::ABS, name: "BIT", cycles: 4 },
        Instruction {opcode: 0x2D, func: Cpu::and,  addr_mode: AddrMode::ABS, name: "AND", cycles: 4 },
        Instruction {opcode: 0x2E, func: Cpu::rol,  addr_mode: AddrMode::ABS, name: "ROL", cycles: 6 },
        Instruction {opcode: 0x2F, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "RLA", cycles: 6 },

        Instruction {opcode: 0x30, func: Cpu::bmi,  addr_mode: AddrMode::REL, name: "BMI", cycles: 2 },
        Instruction {opcode: 0x31, func: Cpu::and,  addr_mode: AddrMode::IZY, name: "AND", cycles: 5 },
        Instruction {opcode: 0x32, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x33, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "RLA", cycles: 8 },
        Instruction {opcode: 0x34, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x35, func: Cpu::and,  addr_mode: AddrMode::ZPX, name: "AND", cycles: 4 },
        Instruction {opcode: 0x36, func: Cpu::rol,  addr_mode: AddrMode::ZPX, name: "ROL", cycles: 6 },
        Instruction {opcode: 0x37, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "RLA", cycles: 6 },
        Instruction {opcode: 0x38, func: Cpu::sec,  addr_mode: AddrMode::IMP, name: "SEC", cycles: 2 },
        Instruction {opcode: 0x39, func: Cpu::and,  addr_mode: AddrMode::ABY, name: "AND", cycles: 4 },
        Instruction {opcode: 0x3A, func: Cpu::oops, addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x3B, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "RLA", cycles: 7 },
        Instruction {opcode: 0x3C, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x3D, func: Cpu::and,  addr_mode: AddrMode::ABX, name: "AND", cycles: 4 },
        Instruction {opcode: 0x3E, func: Cpu::rol,  addr_mode: AddrMode::ABX, name: "ROL", cycles: 7 },
        Instruction {opcode: 0x3F, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "RLA", cycles: 7 },

        Instruction {opcode: 0x40, func: Cpu::rti,  addr_mode: AddrMode::IMP, name: "RTI", cycles: 6 },
        Instruction {opcode: 0x41, func: Cpu::eor,  addr_mode: AddrMode::IZX, name: "EOR", cycles: 6 },
        Instruction {opcode: 0x42, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x43, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "SRE", cycles: 8 },
        Instruction {opcode: 0x44, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "NOP", cycles: 3 },
        Instruction {opcode: 0x45, func: Cpu::eor,  addr_mode: AddrMode::ZP,  name: "EOR", cycles: 3 },
        Instruction {opcode: 0x46, func: Cpu::lsr,  addr_mode: AddrMode::ZP,  name: "LSR", cycles: 5 },
        Instruction {opcode: 0x47, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "SRE", cycles: 5 },
        Instruction {opcode: 0x48, func: Cpu::pha,  addr_mode: AddrMode::IMP, name: "PHA", cycles: 3 },
        Instruction {opcode: 0x49, func: Cpu::eor,  addr_mode: AddrMode::IMM, name: "EOR", cycles: 2 },
        Instruction {opcode: 0x4A, func: Cpu::lsr,  addr_mode: AddrMode::ACC, name: "LSR", cycles: 2 },
        Instruction {opcode: 0x4B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ALR", cycles: 2 },
        Instruction {opcode: 0x4C, func: Cpu::jmp,  addr_mode: AddrMode::ABS, name: "JMP", cycles: 3 },
        Instruction {opcode: 0x4D, func: Cpu::eor,  addr_mode: AddrMode::ABS, name: "EOR", cycles: 4 },
        Instruction {opcode: 0x4E, func: Cpu::lsr,  addr_mode: AddrMode::ABS, name: "LSR", cycles: 6 },
        Instruction {opcode: 0x4F, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "SRE", cycles: 6 },

        Instruction {opcode: 0x50, func: Cpu::bvc,  addr_mode: AddrMode::REL, name: "BVC", cycles: 2 },
        Instruction {opcode: 0x51, func: Cpu::eor,  addr_mode: AddrMode::IZY, name: "EOR", cycles: 5 },
        Instruction {opcode: 0x52, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x53, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "SRE", cycles: 8 },
        Instruction {opcode: 0x54, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x55, func: Cpu::eor,  addr_mode: AddrMode::ZPX, name: "EOR", cycles: 4 },
        Instruction {opcode: 0x56, func: Cpu::lsr,  addr_mode: AddrMode::ZPX, name: "LSR", cycles: 6 },
        Instruction {opcode: 0x57, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "SRE", cycles: 6 },
        Instruction {opcode: 0x58, func: Cpu::cli,  addr_mode: AddrMode::IMP, name: "CLI", cycles: 2 },
        Instruction {opcode: 0x59, func: Cpu::eor,  addr_mode: AddrMode::ABY, name: "EOR", cycles: 4 },
        Instruction {opcode: 0x5A, func: Cpu::oops, addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x5B, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "SRE", cycles: 7 },
        Instruction {opcode: 0x5C, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x5D, func: Cpu::eor,  addr_mode: AddrMode::ABX, name: "EOR", cycles: 4 },
        Instruction {opcode: 0x5E, func: Cpu::lsr,  addr_mode: AddrMode::ABX, name: "LSR", cycles: 7 },
        Instruction {opcode: 0x5F, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "SRE", cycles: 7 },

        Instruction {opcode: 0x60, func: Cpu::rts,  addr_mode: AddrMode::IMP, name: "RTS", cycles: 6 },
        Instruction {opcode: 0x61, func: Cpu::adc,  addr_mode: AddrMode::IZX, name: "ADC", cycles: 6 },
        Instruction {opcode: 0x62, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x63, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "RRA", cycles: 8 },
        Instruction {opcode: 0x64, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "NOP", cycles: 3 },
        Instruction {opcode: 0x65, func: Cpu::adc,  addr_mode: AddrMode::ZP,  name: "ADC", cycles: 3 },
        Instruction {opcode: 0x66, func: Cpu::ror,  addr_mode: AddrMode::ZP,  name: "ROR", cycles: 5 },
        Instruction {opcode: 0x67, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "RRA", cycles: 5 },
        Instruction {opcode: 0x68, func: Cpu::pla,  addr_mode: AddrMode::IMP, name: "PLA", cycles: 4 },
        Instruction {opcode: 0x69, func: Cpu::adc,  addr_mode: AddrMode::IMM, name: "ADC", cycles: 2 },
        Instruction {opcode: 0x6A, func: Cpu::ror,  addr_mode: AddrMode::ACC, name: "ROR", cycles: 2 },
        Instruction {opcode: 0x6B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "ARR", cycles: 2 },
        Instruction {opcode: 0x6C, func: Cpu::jmp,  addr_mode: AddrMode::IND, name: "JMP", cycles: 5 },
        Instruction {opcode: 0x6D, func: Cpu::adc,  addr_mode: AddrMode::ABS, name: "ADC", cycles: 4 },
        Instruction {opcode: 0x6E, func: Cpu::ror,  addr_mode: AddrMode::ABS, name: "ROR", cycles: 6 },
        Instruction {opcode: 0x6F, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "RRA", cycles: 6 },

        Instruction {opcode: 0x70, func: Cpu::bvs,  addr_mode: AddrMode::REL, name: "BVS", cycles: 2 },
        Instruction {opcode: 0x71, func: Cpu::adc,  addr_mode: AddrMode::IZY, name: "ADC", cycles: 5 },
        Instruction {opcode: 0x72, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x73, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "RRA", cycles: 8 },
        Instruction {opcode: 0x74, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x75, func: Cpu::adc,  addr_mode: AddrMode::ZPX, name: "ADC", cycles: 4 },
        Instruction {opcode: 0x76, func: Cpu::ror,  addr_mode: AddrMode::ZPX, name: "ROR", cycles: 6 },
        Instruction {opcode: 0x77, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "RRA", cycles: 6 },
        Instruction {opcode: 0x78, func: Cpu::sei,  addr_mode: AddrMode::IMP, name: "SEI", cycles: 2 },
        Instruction {opcode: 0x79, func: Cpu::adc,  addr_mode: AddrMode::ABY, name: "ADC", cycles: 4 },
        Instruction {opcode: 0x7A, func: Cpu::oops, addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x7B, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "RRA", cycles: 7 },
        Instruction {opcode: 0x7C, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x7D, func: Cpu::adc,  addr_mode: AddrMode::ABX, name: "ADC", cycles: 4 },
        Instruction {opcode: 0x7E, func: Cpu::ror,  addr_mode: AddrMode::ABX, name: "ROR", cycles: 7 },
        Instruction {opcode: 0x7F, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "RRA", cycles: 7 },

        Instruction {opcode: 0x80, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x81, func: Cpu::sta,  addr_mode: AddrMode::IZX, name: "STA", cycles: 6 },
        Instruction {opcode: 0x82, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x83, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "SAX", cycles: 6 },
        Instruction {opcode: 0x84, func: Cpu::sty,  addr_mode: AddrMode::ZP,  name: "STY", cycles: 3 },
        Instruction {opcode: 0x85, func: Cpu::sta,  addr_mode: AddrMode::ZP,  name: "STA", cycles: 3 },
        Instruction {opcode: 0x86, func: Cpu::stx,  addr_mode: AddrMode::ZP,  name: "STX", cycles: 3 },
        Instruction {opcode: 0x87, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "SAX", cycles: 3 },
        Instruction {opcode: 0x88, func: Cpu::dey,  addr_mode: AddrMode::IMP, name: "DEY", cycles: 2 },
        Instruction {opcode: 0x89, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x8A, func: Cpu::txa,  addr_mode: AddrMode::IMP, name: "TXA", cycles: 2 },
        Instruction {opcode: 0x8B, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "XAA", cycles: 2 },
        Instruction {opcode: 0x8C, func: Cpu::sty,  addr_mode: AddrMode::ABS, name: "STY", cycles: 4 },
        Instruction {opcode: 0x8D, func: Cpu::sta,  addr_mode: AddrMode::ABS, name: "STA", cycles: 4 },
        Instruction {opcode: 0x8E, func: Cpu::stx,  addr_mode: AddrMode::ABS, name: "STX", cycles: 4 },
        Instruction {opcode: 0x8F, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "SAX", cycles: 4 },

        Instruction {opcode: 0x90, func: Cpu::bcc,  addr_mode: AddrMode::REL, name: "BCC", cycles: 2 },
        Instruction {opcode: 0x91, func: Cpu::sta,  addr_mode: AddrMode::IZY, name: "STA", cycles: 6 },
        Instruction {opcode: 0x92, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x93, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "AHX", cycles: 6 },
        Instruction {opcode: 0x94, func: Cpu::sty,  addr_mode: AddrMode::ZPX, name: "STY", cycles: 4 },
        Instruction {opcode: 0x95, func: Cpu::sta,  addr_mode: AddrMode::ZPX, name: "STA", cycles: 4 },
        Instruction {opcode: 0x96, func: Cpu::stx,  addr_mode: AddrMode::ZPY, name: "STX", cycles: 4 },
        Instruction {opcode: 0x97, func: Cpu::oops, addr_mode: AddrMode::ZPY, name: "SAX", cycles: 4 },
        Instruction {opcode: 0x98, func: Cpu::tya,  addr_mode: AddrMode::IMP, name: "TYA", cycles: 2 },
        Instruction {opcode: 0x99, func: Cpu::sta,  addr_mode: AddrMode::ABY, name: "STA", cycles: 5 },
        Instruction {opcode: 0x9A, func: Cpu::txs,  addr_mode: AddrMode::IMP, name: "TXS", cycles: 2 },
        Instruction {opcode: 0x9B, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "TAS", cycles: 5 },
        Instruction {opcode: 0x9C, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "SHY", cycles: 5 },
        Instruction {opcode: 0x9D, func: Cpu::sta,  addr_mode: AddrMode::ABX, name: "STA", cycles: 5 },
        Instruction {opcode: 0x9E, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "SHX", cycles: 5 },
        Instruction {opcode: 0x9F, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "AHX", cycles: 5 },

        Instruction {opcode: 0xA0, func: Cpu::ldy,  addr_mode: AddrMode::IMM, name: "LDY", cycles: 2 },
        Instruction {opcode: 0xA1, func: Cpu::lda,  addr_mode: AddrMode::IZX, name: "LDA", cycles: 6 },
        Instruction {opcode: 0xA2, func: Cpu::ldx,  addr_mode: AddrMode::IMM, name: "LDX", cycles: 2 },
        Instruction {opcode: 0xA3, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "LAX", cycles: 6 },
        Instruction {opcode: 0xA4, func: Cpu::ldy,  addr_mode: AddrMode::ZP,  name: "LDY", cycles: 3 },
        Instruction {opcode: 0xA5, func: Cpu::lda,  addr_mode: AddrMode::ZP,  name: "LDA", cycles: 3 },
        Instruction {opcode: 0xA6, func: Cpu::ldx,  addr_mode: AddrMode::ZP,  name: "LDX", cycles: 3 },
        Instruction {opcode: 0xA7, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "LAX", cycles: 3 },
        Instruction {opcode: 0xA8, func: Cpu::tay,  addr_mode: AddrMode::IMP, name: "TAY", cycles: 2 },
        Instruction {opcode: 0xA9, func: Cpu::lda,  addr_mode: AddrMode::IMM, name: "LDA", cycles: 2 },
        Instruction {opcode: 0xAA, func: Cpu::tax,  addr_mode: AddrMode::IMP, name: "TAX", cycles: 2 },
        Instruction {opcode: 0xAB, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "LAX", cycles: 2 },
        Instruction {opcode: 0xAC, func: Cpu::ldy,  addr_mode: AddrMode::ABS, name: "LDY", cycles: 4 },
        Instruction {opcode: 0xAD, func: Cpu::lda,  addr_mode: AddrMode::ABS, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xAE, func: Cpu::ldx,  addr_mode: AddrMode::ABS, name: "LDX", cycles: 4 },
        Instruction {opcode: 0xAF, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "LAX", cycles: 4 },

        Instruction {opcode: 0xB0, func: Cpu::bcs,  addr_mode: AddrMode::REL, name: "BCS", cycles: 2 },
        Instruction {opcode: 0xB1, func: Cpu::lda,  addr_mode: AddrMode::IZY, name: "LDA", cycles: 5 },
        Instruction {opcode: 0xB2, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0xB3, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "LAX", cycles: 5 },
        Instruction {opcode: 0xB4, func: Cpu::ldy,  addr_mode: AddrMode::ZPX, name: "LDY", cycles: 4 },
        Instruction {opcode: 0xB5, func: Cpu::lda,  addr_mode: AddrMode::ZPX, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xB6, func: Cpu::ldx,  addr_mode: AddrMode::ZPY, name: "LDX", cycles: 4 },
        Instruction {opcode: 0xB7, func: Cpu::oops, addr_mode: AddrMode::ZPY, name: "LAX", cycles: 4 },
        Instruction {opcode: 0xB8, func: Cpu::clv,  addr_mode: AddrMode::IMP, name: "CLV", cycles: 2 },
        Instruction {opcode: 0xB9, func: Cpu::lda,  addr_mode: AddrMode::ABY, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xBA, func: Cpu::tsx,  addr_mode: AddrMode::IMP, name: "TSX", cycles: 2 },
        Instruction {opcode: 0xBB, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "LAS", cycles: 4 },
        Instruction {opcode: 0xBC, func: Cpu::ldy,  addr_mode: AddrMode::ABX, name: "LDY", cycles: 4 },
        Instruction {opcode: 0xBD, func: Cpu::lda,  addr_mode: AddrMode::ABX, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xBE, func: Cpu::ldx,  addr_mode: AddrMode::ABY, name: "LDX", cycles: 4 },
        Instruction {opcode: 0xBF, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "LAX", cycles: 4 },

        Instruction {opcode: 0xC0, func: Cpu::cpy,  addr_mode: AddrMode::IMM, name: "CPY", cycles: 2 },
        Instruction {opcode: 0xC1, func: Cpu::cmp,  addr_mode: AddrMode::IZX, name: "CMP", cycles: 6 },
        Instruction {opcode: 0xC2, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "NOP", cycles: 2 },
        Instruction {opcode: 0xC3, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "DCP", cycles: 8 },
        Instruction {opcode: 0xC4, func: Cpu::cpy,  addr_mode: AddrMode::ZP,  name: "CPY", cycles: 3 },
        Instruction {opcode: 0xC5, func: Cpu::cmp,  addr_mode: AddrMode::ZP,  name: "CMP", cycles: 3 },
        Instruction {opcode: 0xC6, func: Cpu::dec,  addr_mode: AddrMode::ZP,  name: "DEC", cycles: 5 },
        Instruction {opcode: 0xC7, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "DCP", cycles: 5 },
        Instruction {opcode: 0xC8, func: Cpu::iny,  addr_mode: AddrMode::IMP, name: "INY", cycles: 2 },
        Instruction {opcode: 0xC9, func: Cpu::cmp,  addr_mode: AddrMode::IMM, name: "CMP", cycles: 2 },
        Instruction {opcode: 0xCA, func: Cpu::dex,  addr_mode: AddrMode::IMP, name: "DEX", cycles: 2 },
        Instruction {opcode: 0xCB, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "AXS", cycles: 2 },
        Instruction {opcode: 0xCC, func: Cpu::cpy,  addr_mode: AddrMode::ABS, name: "CPY", cycles: 4 },
        Instruction {opcode: 0xCD, func: Cpu::cmp,  addr_mode: AddrMode::ABS, name: "CMP", cycles: 4 },
        Instruction {opcode: 0xCE, func: Cpu::dec,  addr_mode: AddrMode::ABS, name: "DEC", cycles: 6 },
        Instruction {opcode: 0xCF, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "DCP", cycles: 6 },

        Instruction {opcode: 0xD0, func: Cpu::bne,  addr_mode: AddrMode::REL, name: "BNE", cycles: 2 },
        Instruction {opcode: 0xD1, func: Cpu::cmp,  addr_mode: AddrMode::IZY, name: "CMP", cycles: 5 },
        Instruction {opcode: 0xD2, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0xD3, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "DCP", cycles: 8 },
        Instruction {opcode: 0xD4, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0xD5, func: Cpu::cmp,  addr_mode: AddrMode::ZPX, name: "CMP", cycles: 4 },
        Instruction {opcode: 0xD6, func: Cpu::dec,  addr_mode: AddrMode::ZPX, name: "DEC", cycles: 6 },
        Instruction {opcode: 0xD7, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "DCP", cycles: 6 },
        Instruction {opcode: 0xD8, func: Cpu::cld,  addr_mode: AddrMode::IMP, name: "CLD", cycles: 2 },
        Instruction {opcode: 0xD9, func: Cpu::cmp,  addr_mode: AddrMode::ABY, name: "CMP", cycles: 4 },
        Instruction {opcode: 0xDA, func: Cpu::oops, addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0xDB, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "DCP", cycles: 7 },
        Instruction {opcode: 0xDC, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0xDD, func: Cpu::cmp,  addr_mode: AddrMode::ABX, name: "CMP", cycles: 4 },
        Instruction {opcode: 0xDE, func: Cpu::dec,  addr_mode: AddrMode::ABX, name: "DEC", cycles: 7 },
        Instruction {opcode: 0xDF, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "DCP", cycles: 7 },

        Instruction {opcode: 0xE0, func: Cpu::cpx,  addr_mode: AddrMode::IMM, name: "CPX", cycles: 2 },
        Instruction {opcode: 0xE1, func: Cpu::sbc,  addr_mode: AddrMode::IZX, name: "SBC", cycles: 6 },
        Instruction {opcode: 0xE2, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "NOP", cycles: 2 },
        Instruction {opcode: 0xE3, func: Cpu::oops, addr_mode: AddrMode::IZX, name: "ISC", cycles: 8 },
        Instruction {opcode: 0xE4, func: Cpu::cpx,  addr_mode: AddrMode::ZP,  name: "CPX", cycles: 3 },
        Instruction {opcode: 0xE5, func: Cpu::sbc,  addr_mode: AddrMode::ZP,  name: "SBC", cycles: 3 },
        Instruction {opcode: 0xE6, func: Cpu::inc,  addr_mode: AddrMode::ZP,  name: "INC", cycles: 5 },
        Instruction {opcode: 0xE7, func: Cpu::oops, addr_mode: AddrMode::ZP,  name: "ISC", cycles: 5 },
        Instruction {opcode: 0xE8, func: Cpu::inx,  addr_mode: AddrMode::IMP, name: "INX", cycles: 2 },
        Instruction {opcode: 0xE9, func: Cpu::sbc,  addr_mode: AddrMode::IMM, name: "SBC", cycles: 2 },
        Instruction {opcode: 0xEA, func: Cpu::nop,  addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0xEB, func: Cpu::oops, addr_mode: AddrMode::IMM, name: "SBC", cycles: 2 },
        Instruction {opcode: 0xEC, func: Cpu::cpx,  addr_mode: AddrMode::ABS, name: "CPX", cycles: 4 },
        Instruction {opcode: 0xED, func: Cpu::sbc,  addr_mode: AddrMode::ABS, name: "SBC", cycles: 4 },
        Instruction {opcode: 0xEE, func: Cpu::inc,  addr_mode: AddrMode::ABS, name: "INC", cycles: 6 },
        Instruction {opcode: 0xEF, func: Cpu::oops, addr_mode: AddrMode::ABS, name: "ISC", cycles: 6 },

        Instruction {opcode: 0xF0, func: Cpu::beq,  addr_mode: AddrMode::REL, name: "BEQ", cycles: 2 },
        Instruction {opcode: 0xF1, func: Cpu::sbc,  addr_mode: AddrMode::IZY, name: "SBC", cycles: 5 },
        Instruction {opcode: 0xF2, func: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0xF3, func: Cpu::oops, addr_mode: AddrMode::IZY, name: "ISC", cycles: 8 },
        Instruction {opcode: 0xF4, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0xF5, func: Cpu::sbc,  addr_mode: AddrMode::ZPX, name: "SBC", cycles: 4 },
        Instruction {opcode: 0xF6, func: Cpu::inc,  addr_mode: AddrMode::ZPX, name: "INC", cycles: 6 },
        Instruction {opcode: 0xF7, func: Cpu::oops, addr_mode: AddrMode::ZPX, name: "ISC", cycles: 6 },
        Instruction {opcode: 0xF8, func: Cpu::sed,  addr_mode: AddrMode::IMP, name: "SED", cycles: 2 },
        Instruction {opcode: 0xF9, func: Cpu::sbc,  addr_mode: AddrMode::ABY, name: "SBC", cycles: 4 },
        Instruction {opcode: 0xFA, func: Cpu::oops, addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0xFB, func: Cpu::oops, addr_mode: AddrMode::ABY, name: "ISC", cycles: 7 },
        Instruction {opcode: 0xFC, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0xFD, func: Cpu::sbc,  addr_mode: AddrMode::ABX, name: "SBC", cycles: 4 },
        Instruction {opcode: 0xFE, func: Cpu::inc,  addr_mode: AddrMode::ABX, name: "INC", cycles: 7 },
        Instruction {opcode: 0xFF, func: Cpu::oops, addr_mode: AddrMode::ABX, name: "ISC", cycles: 7 },
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    
    impl Cpu {
        pub fn default() -> Self {
            Self {
                reg: Registers::default(),
                mem: Memory::default(),
                opcode: 0,
                operand_address: 0,
                operand_value: 0,
                page_penalty: 0,
                extra_cycles: 0,
                cycle_count: 0,
            }
        }
    }


    fn get_cpu_with_mem_ramp() -> Cpu {
        let mut mem = Memory::default();

        for page in 0..256 {
            for byte in 0..256 {
                mem.write(page * 256 + byte, byte as u8);
            }
        }

        let cpu = Cpu::new(mem);

        cpu
    }

    //#[test]
    //fn test_cpu_read_byte2() {
    //    let pc = 0x1000;
    //    let expected: u8 = 42;

    //    let mut cpu = get_cpu();
    //    cpu.reg.PC = pc;
    //    cpu.mem.write(pc, 42);

    //    let byte_read = cpu.read_byte();

    //    assert!(byte_read == expected);
    //}

    #[test]
    fn test_cpu_read_byte() {
        let pc = 0x400; // address 1024 in decimal, page 4

        let mut cpu = get_cpu_with_mem_ramp();
        cpu.reg.PC = pc;

        for i in 0..256 {
            let val = cpu.read_byte();
            println!("read value {:#04x} from address {:#04x}", val, cpu.reg.PC - 1);
            assert!(val == i as u8);
        }
    }

    #[test]
    fn test_cpu_read_word() {
        let pc = 0x2f0; // page 2

        let mut cpu = get_cpu_with_mem_ramp();
        cpu.reg.PC = pc;

        let val = cpu.read_word();

        assert!(val == 61936);
    }

    #[test]
    fn test_fetch_operand_imm() {
        let mut cpu = get_cpu_with_mem_ramp();
        cpu.reg.PC = 0x501;

        cpu.fetch_operand(&AddrMode::IMM);
        assert!(cpu.operand_value == 1);
    }

    #[test]
    fn test_fetch_operand_abs() {
        let mut cpu = get_cpu_with_mem_ramp();
        cpu.reg.PC = 0xff02;

        // Should read 0x0302 as the address word from PC
        // The value at 0x0302 should be 2 in the mem ramp.
        cpu.fetch_operand(&AddrMode::ABS);
        assert!(cpu.operand_value == 2);
    }

    #[test]
    fn test_and_operation_with_zero_result() {
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x0F;
        cpu.operand_value = 0xF0;
        cpu.and();

        assert!(cpu.reg.A == 0);
        assert!(utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_and_operation_with_negative_result() {
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x81;
        cpu.operand_value = 0xF1;
        cpu.and();

        assert!(cpu.reg.A == 0x81);
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_eor_operation() {
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x0F;
        cpu.operand_value = 0xFF;
        cpu.eor();

        assert!(cpu.reg.A == 0xF0);
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_ora_operation() {
        let mut cpu = Cpu::default();
        cpu.reg.A   = 0x8F;
        cpu.operand_value = 0x71;
        cpu.ora();

        assert!(cpu.reg.A == 0xFF);
        assert!(!utils::bit_is_set(PS_Z_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_N_BIT, cpu.reg.P));
    }

    #[test]
    fn test_adc_operation() {
        let mut cpu = Cpu::default();

        cpu.reg.A = 1;
        cpu.operand_value = 1;
        cpu.adc();
        assert!(cpu.reg.A == 2);
        assert!(!utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_V_BIT, cpu.reg.P));

        cpu.clc();
        cpu.reg.A = 1;
        cpu.operand_value = -1i8 as u8;
        cpu.adc();
        assert!(cpu.reg.A == 0);
        assert!(utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(!utils::bit_is_set(PS_V_BIT, cpu.reg.P));

        cpu.clc();
        cpu.reg.A = 0x7F;
        cpu.operand_value = 1;
        cpu.adc();
        assert!(cpu.reg.A == 128);
        assert!(!utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_V_BIT, cpu.reg.P));

        cpu.clc();
        cpu.reg.A = 0x80;
        cpu.operand_value = 0xFF;
        cpu.adc();
        assert!(cpu.reg.A == 0x7F);
        assert!(utils::bit_is_set(PS_C_BIT, cpu.reg.P));
        assert!(utils::bit_is_set(PS_V_BIT, cpu.reg.P));
    }
}