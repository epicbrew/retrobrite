use crate::mem::Memory;

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
    P: u8,  // Status flags
}

///
/// NES 6502 CPU.
/// 
pub struct Cpu {
    /// CPU registers
    reg: Registers,
    
    /// CPU memory
    mem: Memory,

    opcode: u8,
    operand: u8,

    /// CPU cycle counter
    cycle_count: u64,
}

type CpuOp = fn(&mut Cpu);
//type OpFunc = fn(&mut Registers, &mut Memory);

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

struct Instruction {
    opcode: u8,
    op: CpuOp,
    addr_mode: AddrMode,
    name: &'static str,
    cycles: u8,
}

impl Cpu {

    pub fn new(mem: Memory) -> Self {
        let mut new_self = Self {
            reg: Registers::default(),
            mem,
            opcode: 0,
            operand: 0,
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
        //self.reg.PC = self.mem.read_word(0xFFFC);
        self.reg.SP = 0xFD;
        self.reg.P = 0;
    }

    // TODO: pass in a reference to our memory
    pub fn cycle_to(&mut self, cycle: u64) {
        while self.cycle_count < cycle {
            let _instruction = self.read_instruction();
            let cycles_used = self.execute();
            self.cycle_count += cycles_used;
        }
    }

    fn execute(&mut self) -> u64 {
        self.opcode = self.read_instruction();
        self.fetch_operand();

        // TODO: lookup opcode and execute it
        let idx = self.opcode as usize;

        let instruction = &Cpu::OP_CODES[idx];

        (instruction.op)(self);

        instruction.cycles as u64
    }

    //fn read_instruction(&mut self, mem: &mut Memory) -> u8 {
    fn read_instruction(&mut self) -> u8 {
        //mem.write(self.PC + 1, 0);
        let instruction = self.mem.read(self.reg.PC);
        self.reg.PC += 1;

        instruction
    }

    fn fetch_operand(&mut self) {
        // TODO: fetch operand based on opcode addressing mode
    }

    //
    // CPU Instructions
    //
    fn and(&mut self) {
        self.oops();
    }

    fn asl(&mut self) {
        self.oops();
    }

    fn bcs(&mut self) {
        self.oops();
    }

    fn bit(&mut self) {
        self.oops();
    }

    fn brk(&mut self) {
        self.oops();
    }

    fn bpl(&mut self) {
        self.oops();
    }

    fn clc(&mut self) {
        self.oops();
    }

    fn clv(&mut self) {
        self.oops();
    }

    fn jsr(&mut self) {
        self.oops();
    }

    fn lda(&mut self) {
        self.reg.A = self.operand;
    }

    fn ldx(&mut self) {
        self.reg.X = self.operand;
    }

    fn ldy(&mut self) {
        self.reg.Y = self.operand;
    }

    fn ora(&mut self) {
        self.oops();
    }

    fn php(&mut self) {
        self.oops();
    }

    fn plp(&mut self) {
        self.oops();
    }

    fn rol(&mut self) {
        self.oops();
    }

    fn tax(&mut self) {
        self.oops();
    }

    fn tay(&mut self) {
        self.oops();
    }

    fn tsx(&mut self) {
        self.oops();
    }

    fn oops(&mut self) {
        let idx = self.opcode as usize;
        error!("unsupported instruction: opcode: {}, name: {}", self.opcode, Cpu::OP_CODES[idx].name);
        std::process::exit(1);
    }

    const OP_CODES: [Instruction; 256] = [
        Instruction {opcode: 0x00, op: Cpu::brk,  addr_mode: AddrMode::IMP, name: "BRK", cycles: 7 },
        Instruction {opcode: 0x01, op: Cpu::ora,  addr_mode: AddrMode::IZX, name: "ORA", cycles: 6 },
        Instruction {opcode: 0x02, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x03, op: Cpu::oops, addr_mode: AddrMode::IZX, name: "SLO", cycles: 8 },
        Instruction {opcode: 0x04, op: Cpu::oops, addr_mode: AddrMode::ZP,  name: "NOP", cycles: 3 },
        Instruction {opcode: 0x05, op: Cpu::ora,  addr_mode: AddrMode::ZP,  name: "ORA", cycles: 3 },
        Instruction {opcode: 0x06, op: Cpu::asl,  addr_mode: AddrMode::ZP,  name: "ASL", cycles: 5 },
        Instruction {opcode: 0x07, op: Cpu::oops, addr_mode: AddrMode::ZP,  name: "SLO", cycles: 5 },
        Instruction {opcode: 0x08, op: Cpu::php,  addr_mode: AddrMode::IMP, name: "PHP", cycles: 3 },
        Instruction {opcode: 0x09, op: Cpu::ora,  addr_mode: AddrMode::IMM, name: "ORA", cycles: 2 },
        Instruction {opcode: 0x0A, op: Cpu::asl,  addr_mode: AddrMode::ACC, name: "ASL", cycles: 2 },
        Instruction {opcode: 0x0B, op: Cpu::oops, addr_mode: AddrMode::IMM, name: "ANC", cycles: 2 },
        Instruction {opcode: 0x0C, op: Cpu::oops, addr_mode: AddrMode::ABS, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x0D, op: Cpu::ora,  addr_mode: AddrMode::ABS, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x0E, op: Cpu::asl,  addr_mode: AddrMode::ABS, name: "ASL", cycles: 6 },
        Instruction {opcode: 0x0F, op: Cpu::oops, addr_mode: AddrMode::ABS, name: "SLO", cycles: 6 },

        Instruction {opcode: 0x10, op: Cpu::bpl,  addr_mode: AddrMode::REL, name: "BPL", cycles: 2 },
        Instruction {opcode: 0x11, op: Cpu::ora,  addr_mode: AddrMode::IZY, name: "ORA", cycles: 5 },
        Instruction {opcode: 0x12, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x13, op: Cpu::oops, addr_mode: AddrMode::IZY, name: "SLO", cycles: 8 },
        Instruction {opcode: 0x14, op: Cpu::oops, addr_mode: AddrMode::ZPX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x15, op: Cpu::ora,  addr_mode: AddrMode::ZPX, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x16, op: Cpu::asl,  addr_mode: AddrMode::ZPX, name: "ASL", cycles: 6 },
        Instruction {opcode: 0x17, op: Cpu::oops, addr_mode: AddrMode::ZPX, name: "SLO", cycles: 6 },
        Instruction {opcode: 0x18, op: Cpu::clc,  addr_mode: AddrMode::IMP, name: "CLC", cycles: 2 },
        Instruction {opcode: 0x19, op: Cpu::ora,  addr_mode: AddrMode::ABY, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x1A, op: Cpu::oops, addr_mode: AddrMode::IMP, name: "NOP", cycles: 2 },
        Instruction {opcode: 0x1B, op: Cpu::oops, addr_mode: AddrMode::ABY, name: "SLO", cycles: 7 },
        Instruction {opcode: 0x1C, op: Cpu::oops, addr_mode: AddrMode::ABX, name: "NOP", cycles: 4 },
        Instruction {opcode: 0x1D, op: Cpu::ora,  addr_mode: AddrMode::ABX, name: "ORA", cycles: 4 },
        Instruction {opcode: 0x1E, op: Cpu::asl,  addr_mode: AddrMode::ABX, name: "ASL", cycles: 7 },
        Instruction {opcode: 0x1F, op: Cpu::oops, addr_mode: AddrMode::ABX, name: "SLO", cycles: 7 },

        Instruction {opcode: 0x20, op: Cpu::jsr,  addr_mode: AddrMode::ABS, name: "JSR", cycles: 6 },
        Instruction {opcode: 0x21, op: Cpu::and,  addr_mode: AddrMode::IZX, name: "AND", cycles: 6 },
        Instruction {opcode: 0x22, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0x23, op: Cpu::oops, addr_mode: AddrMode::IZX, name: "RLA", cycles: 8 },
        Instruction {opcode: 0x24, op: Cpu::bit,  addr_mode: AddrMode::ZP,  name: "BIT", cycles: 3 },
        Instruction {opcode: 0x25, op: Cpu::and,  addr_mode: AddrMode::ZP,  name: "AND", cycles: 3 },
        Instruction {opcode: 0x26, op: Cpu::rol,  addr_mode: AddrMode::ZP,  name: "ROL", cycles: 5 },
        Instruction {opcode: 0x27, op: Cpu::oops, addr_mode: AddrMode::ZP,  name: "RLA", cycles: 5 },
        Instruction {opcode: 0x28, op: Cpu::plp , addr_mode: AddrMode::IMP, name: "PLP", cycles: 4 },
        Instruction {opcode: 0x29, op: Cpu::and,  addr_mode: AddrMode::IMM, name: "AND", cycles: 2 },
        Instruction {opcode: 0x2A, op: Cpu::rol,  addr_mode: AddrMode::ACC, name: "ROL", cycles: 2 },
        Instruction {opcode: 0x2B, op: Cpu::oops, addr_mode: AddrMode::IMM, name: "ANC", cycles: 2 },
        Instruction {opcode: 0x2C, op: Cpu::bit,  addr_mode: AddrMode::ABS, name: "BIT", cycles: 4 },
        Instruction {opcode: 0x2D, op: Cpu::and,  addr_mode: AddrMode::ABS, name: "AND", cycles: 4 },
        Instruction {opcode: 0x2E, op: Cpu::rol,  addr_mode: AddrMode::ABS, name: "ROL", cycles: 6 },
        Instruction {opcode: 0x2F, op: Cpu::oops, addr_mode: AddrMode::ABS, name: "RLA", cycles: 6 },

        Instruction {opcode: 0x30, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x31, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x32, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x33, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x34, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x35, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x36, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x37, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x38, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x39, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x3A, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x3B, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x3C, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x3D, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x3E, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x3F, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0x40, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x41, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x42, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x43, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x44, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x45, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x46, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x47, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x48, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x49, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x4A, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x4B, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x4C, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x4D, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x4E, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x4F, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0x50, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x51, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x52, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x53, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x54, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x55, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x56, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x57, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x58, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x59, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x5A, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x5B, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x5C, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x5D, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x5E, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x5F, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0x60, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x61, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x62, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x63, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x64, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x65, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x66, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x67, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x68, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x69, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x6A, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x6B, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x6C, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x6D, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x6E, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x6F, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0x70, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x71, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x72, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x73, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x74, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x75, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x76, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x77, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x78, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x79, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x7A, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x7B, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x7C, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x7D, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x7E, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x7F, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0x80, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x81, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x82, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x83, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x84, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x85, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x86, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x87, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x88, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x89, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x8A, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x8B, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x8C, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x8D, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x8E, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x8F, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0x90, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x91, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x92, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x93, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x94, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x95, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x96, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x97, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x98, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x99, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x9A, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x9B, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x9C, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x9D, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x9E, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0x9F, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0xA0, op: Cpu::ldy,  addr_mode: AddrMode::IMM, name: "LDY", cycles: 2 },
        Instruction {opcode: 0xA1, op: Cpu::lda,  addr_mode: AddrMode::IZX, name: "LDA", cycles: 6 },
        Instruction {opcode: 0xA2, op: Cpu::ldx,  addr_mode: AddrMode::IMM, name: "LDX", cycles: 2 },
        Instruction {opcode: 0xA3, op: Cpu::oops, addr_mode: AddrMode::IZX, name: "LAX", cycles: 6 },
        Instruction {opcode: 0xA4, op: Cpu::ldy,  addr_mode: AddrMode::ZP,  name: "LDY", cycles: 3 },
        Instruction {opcode: 0xA5, op: Cpu::lda,  addr_mode: AddrMode::ZP,  name: "LDA", cycles: 3 },
        Instruction {opcode: 0xA6, op: Cpu::ldx,  addr_mode: AddrMode::ZP,  name: "LDX", cycles: 3 },
        Instruction {opcode: 0xA7, op: Cpu::oops, addr_mode: AddrMode::ZP,  name: "LAX", cycles: 3 },
        Instruction {opcode: 0xA8, op: Cpu::tay,  addr_mode: AddrMode::IMP, name: "TAY", cycles: 2 },
        Instruction {opcode: 0xA9, op: Cpu::lda,  addr_mode: AddrMode::IMM, name: "LDA", cycles: 2 },
        Instruction {opcode: 0xAA, op: Cpu::tax,  addr_mode: AddrMode::IMP, name: "TAX", cycles: 2 },
        Instruction {opcode: 0xAB, op: Cpu::oops, addr_mode: AddrMode::IMM, name: "LAX", cycles: 2 },
        Instruction {opcode: 0xAC, op: Cpu::ldy,  addr_mode: AddrMode::ABS, name: "LDY", cycles: 4 },
        Instruction {opcode: 0xAD, op: Cpu::lda,  addr_mode: AddrMode::ABS, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xAE, op: Cpu::ldx,  addr_mode: AddrMode::ABS, name: "LDX", cycles: 4 },
        Instruction {opcode: 0xAF, op: Cpu::oops, addr_mode: AddrMode::ABS, name: "LAX", cycles: 4 },

        Instruction {opcode: 0xB0, op: Cpu::bcs,  addr_mode: AddrMode::REL, name: "BCS", cycles: 2 },
        Instruction {opcode: 0xB1, op: Cpu::lda,  addr_mode: AddrMode::IZY, name: "LDA", cycles: 5 },
        Instruction {opcode: 0xB2, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "ILL", cycles: 0 },
        Instruction {opcode: 0xB3, op: Cpu::oops, addr_mode: AddrMode::IZY, name: "LAX", cycles: 5 },
        Instruction {opcode: 0xB4, op: Cpu::ldy,  addr_mode: AddrMode::ZPX, name: "LDY", cycles: 4 },
        Instruction {opcode: 0xB5, op: Cpu::lda,  addr_mode: AddrMode::ZPX, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xB6, op: Cpu::ldx,  addr_mode: AddrMode::ZPY, name: "LDX", cycles: 4 },
        Instruction {opcode: 0xB7, op: Cpu::oops, addr_mode: AddrMode::ZPY, name: "LAX", cycles: 4 },
        Instruction {opcode: 0xB8, op: Cpu::clv,  addr_mode: AddrMode::IMP, name: "CLV", cycles: 2 },
        Instruction {opcode: 0xB9, op: Cpu::lda,  addr_mode: AddrMode::ABY, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xBA, op: Cpu::tsx,  addr_mode: AddrMode::IMP, name: "TSX", cycles: 2 },
        Instruction {opcode: 0xBB, op: Cpu::oops, addr_mode: AddrMode::ABY, name: "LAS", cycles: 4 },
        Instruction {opcode: 0xBC, op: Cpu::ldy,  addr_mode: AddrMode::ABX, name: "LDY", cycles: 4 },
        Instruction {opcode: 0xBD, op: Cpu::lda,  addr_mode: AddrMode::ABX, name: "LDA", cycles: 4 },
        Instruction {opcode: 0xBE, op: Cpu::ldx,  addr_mode: AddrMode::ABY, name: "LDX", cycles: 4 },
        Instruction {opcode: 0xBF, op: Cpu::oops, addr_mode: AddrMode::ABY, name: "LAX", cycles: 4 },

        Instruction {opcode: 0xC0, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC1, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC2, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC3, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC4, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC5, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC6, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC7, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC8, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xC9, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xCA, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xCB, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xCC, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xCD, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xCE, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xCF, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0xD0, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD1, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD2, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD3, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD4, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD5, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD6, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD7, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD8, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xD9, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xDA, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xDB, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xDC, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xDD, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xDE, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xDF, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0xE0, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE1, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE2, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE3, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE4, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE5, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE6, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE7, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE8, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xE9, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xEA, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xEB, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xEC, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xED, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xEE, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xEF, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },

        Instruction {opcode: 0xF0, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF1, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF2, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF3, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF4, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF5, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF6, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF7, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF8, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xF9, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xFA, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xFB, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xFC, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xFD, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xFE, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
        Instruction {opcode: 0xFF, op: Cpu::oops, addr_mode: AddrMode::UNK, name: "XXX", cycles: 0 },
    ];
}