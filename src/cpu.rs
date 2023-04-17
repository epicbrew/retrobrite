use crate::mem::Memory;

///
/// NES 6502 CPU.
/// 
/// Field names violate the snake case convention due to the convention of
/// referring to the 6502 CPU registers as capital letters in all of the 6502
/// documentation.
/// 
#[allow(non_snake_case)]
pub struct Cpu {
    A: u8,  // Accumulator register
    X: u8,  // X index register
    Y: u8,  // Y index register
    PC: u16,// Program Counter
    SP: u8, // Stack pointer
    P: u8,  // Status flags

    /// CPU cycle counter
    cycle_count: u64,
}

impl Default for Cpu {
    ///
    /// Default CPU constructor.
    /// Using powerup state documented here:
    ///     https://www.nesdev.org/wiki/CPU_power_up_state
    /// 
    fn default() -> Self {
        Self {
            A: 0,
            X: 0,
            Y: 0,
            PC: 0xFFFC,
            SP: 0xFD,
            P: 0x34,
            /// Keep track of how many cycles the cpu has executed.
            cycle_count: 0,
        }
    }
}

impl Cpu {
    ///
    /// Reset CPU as if NES reset button was pressed.
    /// TODO: reset registers to correct values.
    pub fn reset(&mut self) {
        self.A = 0;
        self.X = 0;
        self.Y = 0;
        self.PC = 0xFFFC;
        self.SP = 0xFD;
        self.P = 0;
    }

    // TODO: pass in a reference to our memory
    pub fn cycle_to(&mut self, cycle: u64, mem: &mut Memory) {
        while self.cycle_count < cycle {
            let _instruction = self.read_instruction(mem);
            let cycles_used = self.execute();
            self.cycle_count += cycles_used;
        }
    }

    fn execute(&mut self) -> u64 {
        1
    }

    fn read_instruction(&mut self, mem: &mut Memory) -> u8 {
        //mem.write(self.PC + 1, 0);
        let instruction = mem.read(self.PC);
        self.PC += 1;

        instruction
    }
}