
///
/// NES 6502 CPU.
/// 
/// Field names violate the snake case convention due to the convention of
/// referring to the 6502 CPU registers as capital letters in all of the 6502
/// documentation.
/// 
#[allow(non_snake_case)]
pub struct Cpu {
    A: u8,
    X: u8,
    Y: u8,
    PC: u16,
    S: u8,
    P: u8,
    cycle_count: u64,
}

impl Default for Cpu {
    ///
    /// Default CPU constructor.
    /// 
    fn default() -> Self {
        Self {
            A: 0,
            X: 0,
            Y: 0,
            PC: 0,
            S: 0,
            P: 0,
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
        self.PC = 0;
        self.S = 0;
        self.P = 0;
    }

    // TODO: pass in a reference to our memory
    pub fn cycle_to(&mut self, cycle: u64) {
        while self.cycle_count < cycle {
            let cycles_used = self.execute();
            self.cycle_count += cycles_used;
        }
    }

    fn execute(&mut self) -> u64 {
        1
    }
}