#[macro_use]
extern crate log;
extern crate clap;
extern crate sdl2;

use std::cell::RefCell;
use std::fs;
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
mod wram;

mod state;
use state::NesState;

mod ines;
use ines::InesRom;

use crate::gui::Gui;
use crate::mem::{Memory, PpuMemory};
mod mappers;

mod gui;

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

    /// Number of CPU cycles to run at a time.
    /// Not recommended to change from default.
    #[arg(long, short('b'))]
    cycle_batch: Option<u64>,

    /// Print information about a ROM and exit.
    #[arg(long, short('i'))]
    rom_info: bool,

    /// Enable trace logging of CPU execution.
    #[arg(long)]
    trace_cpu: bool,
}

fn ensure_retrobrite_data_dir_exists() {
    let retrobrite_dir_path = utils::get_data_dir_path();

    if !retrobrite_dir_path.exists() {
        match fs::create_dir_all(&retrobrite_dir_path) {
            Ok(_) => info!("created {}", retrobrite_dir_path.to_string_lossy()),
            Err(e) => error!("could not create {}: {}", 
                                    retrobrite_dir_path.to_string_lossy(), e),
        }
    }
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    let rom_path = cli.rom.expect("No rom specified (try --help)");
    let ines_file = InesRom::from_path(rom_path.as_path());

    if cli.rom_info {
        println!("Rom file: {}", rom_path.display());
        println!("Mapper: {}", ines_file.get_mapper_number());
        println!("{:#?}", ines_file.header);
        println!("sizeof chr-rom: {}", ines_file.chr_rom.len());
        std::process::exit(0);
    }

    ensure_retrobrite_data_dir_exists();

    // Init mapper and load rom
    let mut mapper = mappers::get_mapper(
        ines_file.get_mapper_number(), Memory::new_cpu(), PpuMemory::new());

    mapper.print_info();
    mapper.load_rom(&ines_file);

    // Init state object
    let ppu = Rc::new(RefCell::new(Ppu::new()));
    let mut state = NesState::new(mapper, Rc::clone(&ppu));

    let mut cpu = Cpu::new(&mut state, cli.trace_cpu);

    let max_cycles = if let Some(cycles_to_run) = cli.cycles.as_ref() {
        *cycles_to_run
    } else {
        0
    };

    if let Some(pc) = cli.pc.as_ref() {
        cpu.set_program_counter(*pc);
    }

    let cycle_batch = if let Some(cycle_batch_option) = cli.cycle_batch.as_ref() {
        *cycle_batch_option
    } else {
        1
    };
    let mut cycle = 7;
    let mut cycles_this_second = 0;
    let mut last_report = Instant::now();

    let mut frame_start = Instant::now();
    let frame_duration = Duration::from_nanos(CPU_CYCLES_PER_FRAME * NS_PER_CYCLE);
    let mut frame_count: u64 = 0;
    let mut fps: u32 = 0;

    info!("CPU FREQ: {}", CPU_FREQ);
    info!("ns per cycle: {}", NS_PER_CYCLE);
    info!("cycle_batch: {}", cycle_batch);
    info!("reset vector: {:04X}", state.cpu_mem_read_word(0xFFFC));

    let mut gui = Gui::init().unwrap();

    'mainloop: loop {

        cycle += cycle_batch;

        // Clip cycle to max_cycles if necessary
        if max_cycles > 0 && cycle > max_cycles {
            break 'mainloop;
        }

        let cpu_cyles_used = cpu.cycle_to(&mut state, cycle);

        let mut ppu_ref = ppu.borrow_mut();

        for _ in 0..cpu_cyles_used*3 {
            let ppu_result = ppu_ref.cycle(&mut state);

            match ppu_result {
                ppu::PpuCycleResult::Idle => (),
                ppu::PpuCycleResult::Pixel { scanline, x, color } => {
                    gui.set_pixel(x, scanline, color);
                },
                ppu::PpuCycleResult::HBlank => (),
                ppu::PpuCycleResult::PostRenderLine => (),
                ppu::PpuCycleResult::VBlankLine { trigger_nmi, scanline} => {
                    if trigger_nmi {
                        cpu.set_nmi_flag();
                        trace!("VBlank: nmi triggered at scanline {scanline}");
                    }
                }
                ppu::PpuCycleResult::PreRenderLine { scanline_cycle } => {
                    // Use prerender line scanline cycle 2 as our "sleep point" to
                    // keep timing at 60fps
                    if scanline_cycle == 2 {
                        gui.render_frame();
                        frame_count += 1;
                        fps += 1;

                        if gui.process_events(&mut state) == false {
                            break 'mainloop;
                        }

                        let frame_time_used = Instant::now() - frame_start;
            
                        if frame_time_used < frame_duration {
                          let sleep_time = frame_duration - frame_time_used;
                          sleep(sleep_time);
                        }

                        frame_start = Instant::now();
                    }
                },
            }
        }

        cycles_this_second += cpu_cyles_used;

        if last_report.elapsed().as_millis() >= 1000 {
            info!("elapsed time for 1s cycle: {}ms, cycles this second: {}, fps: {}, frame counter: {}",
                  last_report.elapsed().as_millis(), cycles_this_second, fps, frame_count);

            last_report = Instant::now();
            cycles_this_second = 0;
            fps = 0;
        }
    }

    // Call state shutdown to ensure any shutdown related tasks/bookkeeping are done.
    state.shutdown();
}
