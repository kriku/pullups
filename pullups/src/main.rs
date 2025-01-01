#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use panic_halt as _;

// use embedded_hal::delay::DelayNs;
use waveshare_rp2040_zero::entry;
use waveshare_rp2040_zero::hal::Clock;
use waveshare_rp2040_zero::hal::{clocks, pac, timer, watchdog::Watchdog, Sio};

use tm1637::TM1637;

#[entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    // let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // The single-cycle I/O block controls our GPIO pins
    let sio = Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = waveshare_rp2040_zero::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Set the LED to be an output
    let mut clk = pins.gp3.into_push_pull_output();
    let mut dio = pins.gp2.into_push_pull_output();

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock

    let clocks = clocks::init_clocks_and_plls(
        waveshare_rp2040_zero::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // The delay object lets us wait for specified amounts of time (in
    // milliseconds)
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let mut timer = timer::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let mut tm = TM1637::new(&mut clk, &mut dio, &mut delay);
    tm.init().unwrap(); // append `.unwrap()` to catch and handle exceptions in cost of extra ROM size
    tm.clear().unwrap();
    tm.set_brightness(0xff).unwrap();

    loop {
        for i in 0..9999u16 {
            let mut digits: [u8; 4] = [0; 4];
            digits[0] = (i / 1000).to_be_bytes()[1];
            digits[1] = ((i % 1000) / 100).to_be_bytes()[1];
            digits[2] = ((i % 100) / 10).to_be_bytes()[1];
            digits[3] = (i % 10).to_be_bytes()[1];

            tm.print_hex(0, &digits).unwrap();
            // tm.print_raw(3, &[i]).unwrap();
            // tm.set_brightness(i >> 5).unwrap();

            timer.delay_ms(100);
        }
    }
}
