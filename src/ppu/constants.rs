
/// Size of Object Attribute Memory.
pub const OAM_SIZE: usize = 256;

// Size of secondary OAM.
//pub const SECONDARY_OAM_SIZE: usize = 32;

pub const NAMETABLE_0: u16 = 0x2000;
pub const NAMETABLE_0_END: u16 = NAMETABLE_0 + 0x03FF;

pub const NAMETABLE_1: u16 = 0x2400;
pub const NAMETABLE_1_END: u16 = NAMETABLE_1 + 0x03FF;

pub const NAMETABLE_2: u16 = 0x2800;
pub const NAMETABLE_2_END: u16 = NAMETABLE_2 + 0x03FF;

pub const NAMETABLE_3: u16 = 0x2C00;
pub const NAMETABLE_3_END: u16 = NAMETABLE_3 + 0x03FF;

/// Mirroring type used for PPU memory. PPU can mirror
/// either VRAM bank into multiple nametables.
#[derive(Debug, Clone, Copy)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    OneScreen0,
    OneScreen1,
}
