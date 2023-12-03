#[macro_use]
extern crate log;
extern crate clap;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Instant, Duration};
use std::thread::sleep;
use std::path::PathBuf;
use clap::Parser;

mod utils;
mod cpu;
use cpu::Cpu;

mod ppu;
use ppu::Ppu;

mod mem;

mod state;
use state::NesState;

mod ines;
use ines::InesRom;

use crate::mappers::Mapper;

mod mappers;

const MASTER_CLOCK_HZ: u64 = 21_441_960;
const CLOCK_DIVISOR: u64 = 12;
const CPU_FREQ: u64 = MASTER_CLOCK_HZ / CLOCK_DIVISOR;
const CPU_CYCLES_PER_FRAME: u64 = CPU_FREQ / 60;
const NS_PER_CYCLE: u64 = (1.0 / CPU_FREQ as f64 * 1e9) as u64;
const _NS_PER_FRAME: u64 = CPU_CYCLES_PER_FRAME * NS_PER_CYCLE;

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

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    let rom_path = cli.rom.expect("No rom specifiec (try --help)");
    let ines_file = InesRom::from_path(rom_path.as_path());

    if cli.rom_info {
        println!("Rom file: {}", rom_path.display());
        println!("{:#?}", ines_file.header);
        println!("sizeof chr-rom: {}", ines_file.chr_rom.len());
        std::process::exit(0);
    }

    let mut mapper = mappers::get_mapper(ines_file.get_mapper_number());

    let ppu = Rc::new(RefCell::new(Ppu::new()));
    let mut mc = NesState::new(Rc::clone(&ppu));

    mapper.load_rom(&mut mc, &ines_file);

    mapper.print_info();

    let mut cpu = Cpu::new(&mc);

    let max_cycles = if let Some(cycles_to_run) = cli.cycles.as_ref() {
        *cycles_to_run
    } else {
        0
    };

    if let Some(pc) = cli.pc.as_ref() {
        cpu.set_program_counter(*pc);
    }

    let cycle_batch = CPU_CYCLES_PER_FRAME;
    let mut cycle = 7;
    let mut cycles_this_second = 0;
    let mut last_report = Instant::now();

    let frame_duration = Duration::from_nanos(cycle_batch * NS_PER_CYCLE);
    let mut frame_count: u64 = 0;
    let mut last_report_frame_count = frame_count;

    info!("CPU FREQ: {}", CPU_FREQ);
    info!("ns per cycle: {}", NS_PER_CYCLE);
    info!("cycle_batch: {}", cycle_batch);

    'mainloop: loop {
        let mut cycles_this_frame: u64 = 0;

        let frame_start = Instant::now();

        while cycles_this_frame < cycle_batch {
            cycle += 1;

            // Clip cycle to max_cycles if necessary
            if max_cycles > 0 && cycle > max_cycles {
                break 'mainloop;
            }

            cpu.cycle_to(&mut mc, cycle);

            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.cycle();
            ppu_ref.cycle();
            ppu_ref.cycle();

            cycles_this_frame += 1;
        }

        cycles_this_second += cycles_this_frame;
        frame_count += 1;
        
        let frame_time_used = Instant::now() - frame_start;

        if frame_time_used < frame_duration {
          let sleep_time = frame_duration - frame_time_used;
          sleep(sleep_time);
          //info!("last sleep_time: {}ms", sleep_time.as_millis());
        }

        if cycles_this_second >= CPU_FREQ {
            let fps = frame_count - last_report_frame_count;

            info!("elapsed time for 1s cycle: {}ms, cycles this second: {}, fps: {}",
                  last_report.elapsed().as_millis(), cycles_this_second, fps);

            last_report = Instant::now();
            cycles_this_second = 0;
            last_report_frame_count = frame_count;
        }
    }
}
