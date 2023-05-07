#![allow(unused)]

//
// Processor status flag bits
//

/// Bit location of processor status carry flag (C).
pub const PS_C_BIT: u8 = 0;

/// Bit location of processor status zero flag (Z).
pub const PS_Z_BIT: u8 = 1;

/// Bit location of processor status interupt disable flag (I).
pub const PS_I_BIT: u8 = 2;

/// Bit location of processor status decimal mode flag (D).
pub const PS_D_BIT: u8 = 3;

/// Bit location of processor status break flag (B).
pub const PS_B_BIT: u8 = 4;

/// Bit location of processor status overflow flag (V).
pub const PS_V_BIT: u8 = 6;

/// Bit location of processor status negative flag (N).
pub const PS_N_BIT: u8 = 7;

///// Processor status carry flag mask.
//pub const PS_C_FLAG_MASK: u8 = 0b00000001;
///// Processor status zero flag mask.
//pub const PS_Z_FLAG_MASK: u8 = 0b00000010;
///// Processor status interupt disable flag mask.
//pub const PS_I_FLAG_MASK: u8 = 0b00000100;
///// Processor status decimal flag mask (unused).
//pub const PS_D_FLAG_MASK: u8 = 0b00001000;
///// Processor status break flag mask.
//pub const PS_B_FLAG_MASK: u8 = 0b00010000;
///// Processor status overflow flag mask.
//pub const PS_V_FLAG_MASK: u8 = 0b01000000;
///// Processor status negative flag mask.
//pub const PS_N_FLAG_MASK: u8 = 0b10000000;