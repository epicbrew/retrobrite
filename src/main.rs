#[macro_use]
extern crate log;
extern crate clap;
extern crate ines;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Instant, Duration};
use std::thread::sleep;
use std::path::{PathBuf, Path};
use std::fs;
use clap::Parser;
use ines::Ines;

mod utils;
mod cpu;
use cpu::Cpu;

mod ppu;
use ppu::Ppu;

mod mem;
use mem::MemController;

use crate::mappers::Mapper;

mod mappers;

const MASTER_CLOCK_HZ: u64 = 21_441_960;
const CLOCK_DIVISOR: u64 = 12;
const CPU_FREQ: u64 = MASTER_CLOCK_HZ / CLOCK_DIVISOR;
const NS_PER_CYCLE: u64 = (1.0 / CPU_FREQ as f64 * 1e9) as u64;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Rom file to load (ines format only).
    rom: Option<PathBuf>,

    /// Start program counter at given value (for debug/testing).
    #[arg(long)]
    pc: Option<u16>,

    /// Number of CPU cycles to run (for debug/testing).
    #[arg(long, short)]
    cycles: Option<u64>,

    /// Print information about a ROM and exit.
    #[arg(long, short('i'))]
    rom_info: bool,
}

fn parse_rom_file(rom_path: &Path) -> Ines {
    let file_data = fs::read(&rom_path).expect("could not read rom file");

    match Ines::parse(&file_data) {
        Ok(ines) => ines,
        Err(why) => match why {
            ines::Error::UnimplementedMapper { code } => {
                 panic!("unimplemented mapper: {}", code)
            }
        }
    }
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    let rom_path = cli.rom.expect("No rom specifiec (try --help)");
    let ines_file = parse_rom_file(rom_path.as_path());

    if cli.rom_info {
        println!("Rom file: {}", rom_path.display());
        println!("{:?}", ines_file.header);
        std::process::exit(0);
    }

    let mut mapper = mappers::get_mapper(ines_file.header.mapper as u16);

    let mut mc = MemController::new(mapper.get_observer(),
                                    Rc::new(RefCell::new(Ppu::new())));

    mapper.load_rom(&mut mc, &ines_file);
    //load_prg_rom(&mut mc, &ines_file);

    mapper.print_info();
    info!("reset vector: {:04X}", mc.raw_cpu_mem_read_word(0xFFFC));

    let mut cpu = Cpu::new(&mc);

    let max_cycles = if let Some(cycles_to_run) = cli.cycles.as_ref() {
        *cycles_to_run
    } else {
        0
    };

    if let Some(pc) = cli.pc.as_ref() {
        cpu.set_program_counter(*pc);
    }

    let cycle_batch = 42;
    let mut cycle = 7;
    let mut cycles_this_second = 0;
    let startup_time = Instant::now();
    let mut last_report = Instant::now();

    info!("CPU FREQ: {}", CPU_FREQ);
    info!("ns per cycle: {}", NS_PER_CYCLE);
    info!("cycle_batch: {}", cycle_batch);

    loop {
        cycle += cycle_batch;
        cycles_this_second += cycle_batch;

        // Clip cycle to max_cycles if necessary
        if max_cycles > 0 && cycle > max_cycles {
            cycle = max_cycles;
        }

        cpu.cycle_to(&mut mc, cycle);

        let next_cycle_offset = Duration::from_nanos(cycle * NS_PER_CYCLE); 
        let next_cycle_time = startup_time + next_cycle_offset;
        let sleep_time = next_cycle_time - Instant::now();

        sleep(sleep_time);

        //if cycle % CPU_FREQ == 0 {
        if cycles_this_second >= CPU_FREQ {
            info!("elapsed time for 1s cycle: {}ms, cycles this second: {}",
                  last_report.elapsed().as_millis(), cycles_this_second);
            debug!("last sleep_time: {}ns", sleep_time.as_nanos());
            last_report = Instant::now();
            cycles_this_second = 0;
        }

        if max_cycles > 0 && cycle >= max_cycles {
            break;
        }
        //if let Some(cycles_to_run) = cli.cycles.as_ref() {
        //    if cycle >= *cycles_to_run {
        //        break;
        //    }
        //}

    }
}
