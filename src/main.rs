#[macro_use]
extern crate log;
use std::time::{Instant, Duration};
use std::thread::sleep;

const MASTER_CLOCK_HZ: u64 = 21_441_960;
const CLOCK_DIVISOR: u64 = 12;
const CPU_FREQ: u64 = MASTER_CLOCK_HZ / CLOCK_DIVISOR;
const NS_PER_CYCLE: u64 = (1.0 / CPU_FREQ as f64 * 1e9) as u64;

fn main() {
    env_logger::init();

    let cycle_batch = 42;
    let mut cycle = 0;
    let mut cycles_this_second = 0;
    let startup_time = Instant::now();
    let mut last_report = Instant::now();

    info!("CPU FREQ: {}", CPU_FREQ);
    info!("ns per cycle: {}", NS_PER_CYCLE);
    info!("cycle_batch: {}", cycle_batch);

    loop {
        cycle += cycle_batch;
        cycles_this_second += cycle_batch;

        let next_cycle_offset = Duration::from_nanos(cycle * NS_PER_CYCLE); 
        let next_cycle_time = startup_time + next_cycle_offset;
        let sleep_time = next_cycle_time - Instant::now();

        sleep(sleep_time);

        // simulate work

        //if cycle % CPU_FREQ == 0 {
        if cycles_this_second >= CPU_FREQ {
            info!("elapsed time for 1s cycle: {}ms, cycles this second: {}",
                  last_report.elapsed().as_millis(), cycles_this_second);
            debug!("last sleep_time: {}ns", sleep_time.as_nanos());
            last_report = Instant::now();
            cycles_this_second = 0;
        }
    }
}
