use std::{path::Path, fs};
use crate::utils::bit_is_set;

const HEADER_SIZE: usize = 16;
//                             "N" . "E" . "S"
const NES_FILE_ID: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

/** Size of each PRG ROM chunk. */
pub const PRG_ROM_CHUNK_SIZE: usize = 16384;

/** Offset to num prg rom chunks in header. */
const PRG_ROM_CHUNKS_OFFSET: usize = 4;

/** Size of each CHR ROM chunk. */
pub const CHR_ROM_CHUNK_SIZE: usize = 8192;

/** Offset to num chr rom chunks in header. */
const CHR_ROM_CHUNKS_OFFSET: usize = 5;

#[derive(Debug)]
pub enum FileFormat {
    INES,
    INES2,
}

#[derive(Debug)]
pub enum MirroringType {
    Horizontal,
    Vertical,
}

/**
 * Fields derived from the "Flags 6" byte of the header.
 */
#[derive(Debug)]
pub struct Flags6 {
    pub mirroring: MirroringType,
    pub _has_battery_backed_prg_ram: bool,
    pub has_trainer: bool,
    pub _four_screen_vram: bool,
    pub mapper_lower_nybble: u8,
}

/**
 * Fields derived from the "Flags 7" byte of the header.
 */
#[derive(Debug)]
pub struct Flags7 {
    pub _vs_unisystem: bool,
    pub _playchoice_10: bool,
    pub _file_format: FileFormat,
    pub mapper_upper_nybble: u8,
}

/**
 * Ines file header data.
 */
#[derive(Debug)]
pub struct InesHeader {
    //file_id: [u8; 4],
    pub num_prg_rom_chunks: usize,
    pub num_chr_rom_chunks: usize,
    
    /** Fields from "Flags 6". */
    pub flags6: Flags6,

    /** Fields from "Flags 7". */
    pub flags7: Flags7,
}

/**
 * Basic representation of an INES file.
 */
#[derive(Debug)]
pub struct InesRom {
    pub header: InesHeader,
    pub prg_rom: Vec<[u8; PRG_ROM_CHUNK_SIZE]>,
    pub chr_rom: Vec<[u8; CHR_ROM_CHUNK_SIZE]>,
}

impl Flags6 {
    pub fn parse(flags: u8) -> Self {

        let mapper_lower_nybble = flags >> 4;

        let mirroring = if bit_is_set(0, flags) {
            MirroringType::Vertical
        } else {
            MirroringType::Horizontal
        };

        Self {
            mirroring,
            _has_battery_backed_prg_ram: bit_is_set(1, flags),
            has_trainer: bit_is_set(2, flags),
            _four_screen_vram: bit_is_set(3, flags),
            mapper_lower_nybble,
        }
    }
}

impl Flags7 {
    pub fn parse(flags: u8) -> Self {
        let format_value: u8 = (flags & 0x6) >> 1;

        let file_format = match format_value {
            2 => FileFormat::INES2,
            _ => FileFormat::INES,
        };
            
        Self {
            _vs_unisystem: bit_is_set(0, flags),
            _playchoice_10: bit_is_set(1, flags),
            _file_format: file_format,
            mapper_upper_nybble: flags & 0xf0,
        }
    }
}

impl InesHeader {
    fn parse(buffer: &[u8]) -> Self {
        if buffer[0..4] != NES_FILE_ID {
            panic!("nesrom: invalid file identification");
        }

        let num_prg_rom_chunks = usize::from(buffer[PRG_ROM_CHUNKS_OFFSET]);
        let num_chr_rom_chunks = usize::from(buffer[CHR_ROM_CHUNKS_OFFSET]);

        Self {
            //file_id: buffer[0..4],
            num_prg_rom_chunks,
            num_chr_rom_chunks,
            flags6: Flags6::parse(buffer[6]),
            flags7: Flags7::parse(buffer[7]),
        }
    }
}

///
/// Parses PRG rom buffer into a vec of PRG rom banks. Buffer
/// should be just the slice that contains the PRG rom data.
/// 
fn parse_prg_rom(buffer: &[u8], num_prg_rom_chunks: usize) -> Vec<[u8; PRG_ROM_CHUNK_SIZE]> {
    let mut prg_rom_banks = Vec::new();

    for i in 0..num_prg_rom_chunks {
        let bank_start = i * PRG_ROM_CHUNK_SIZE;
        let bank_end = bank_start + PRG_ROM_CHUNK_SIZE;

        let mut bank = [0; PRG_ROM_CHUNK_SIZE];
        bank.copy_from_slice(&buffer[bank_start..bank_end]);

        prg_rom_banks.push(bank);
    }

    prg_rom_banks
}

///
/// Parses CHR rom buffer into a vec of CHR rom banks. Buffer
/// should be just the slice that contains the CHR rom data.
/// 
fn parse_chr_rom(buffer: &[u8], num_chr_rom_chunks: usize) -> Vec<[u8; CHR_ROM_CHUNK_SIZE]> {
    let mut chr_rom_banks = Vec::new();

    for i in 0..num_chr_rom_chunks {
        let bank_start = i * CHR_ROM_CHUNK_SIZE;
        let bank_end = bank_start + CHR_ROM_CHUNK_SIZE;

        let mut bank = [0; CHR_ROM_CHUNK_SIZE];
        bank.copy_from_slice(&buffer[bank_start..bank_end]);

        chr_rom_banks.push(bank);
    }

    chr_rom_banks
}

impl InesRom {
    pub fn from_buffer(buffer: &[u8]) -> Self {
        let header = InesHeader::parse(buffer);

        let trainer_bytes: usize = if header.flags6.has_trainer { 512 } else { 0 };

        let prg_rom_offset = HEADER_SIZE + trainer_bytes;
        let prg_rom_size = PRG_ROM_CHUNK_SIZE * header.num_prg_rom_chunks;

        let chr_rom_offset = prg_rom_offset + prg_rom_size;
        let chr_rom_size = CHR_ROM_CHUNK_SIZE * header.num_chr_rom_chunks;

        let prg_rom = parse_prg_rom(
            &buffer[prg_rom_offset..prg_rom_offset + prg_rom_size],
            header.num_prg_rom_chunks);

        let chr_rom = parse_chr_rom(
            &buffer[chr_rom_offset..chr_rom_offset + chr_rom_size],
            header.num_chr_rom_chunks);

        Self {
            header,
            prg_rom,
            chr_rom,
        }
    }

    pub fn from_path(path: &Path) -> Self {
        let file_data = fs::read(path).expect("could not read rom file");

        Self::from_buffer(&file_data)
    }

    pub fn get_mapper_number(&self) -> u16 {
        let mapper: u8 = 
            self.header.flags7.mapper_upper_nybble | self.header.flags6.mapper_lower_nybble;

        mapper as u16
    }
}

/*
fn bit_is_set(bit: u8, input: u8) -> bool {
    (input & (1 << bit)) != 0
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn it_works() {
        let path_buf = PathBuf::from(r"./nestest/nestest.nes");
        let nesrom = InesRom::from_path(&path_buf.as_path());
        println!("{:#?}", nesrom.header);
        println!("prg_rom_size: {}", nesrom.prg_rom.len());
        println!("chr_rom_size: {}", nesrom.chr_rom.len());
    }
}
