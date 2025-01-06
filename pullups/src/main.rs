#![no_std]
#![no_main]

use embedded_hal::digital::{InputPin, OutputPin};
use panic_halt as _;

use waveshare_rp2040_zero::entry;
use waveshare_rp2040_zero::hal::Clock;
use waveshare_rp2040_zero::hal::{clocks, pac, timer, watchdog::Watchdog, Sio};

use tm1637::TM1637;

const PULSE_INTERVAL: u64 = 250_000;
const PULSE_LENGTH: u64 = 20_000;
const HOLD_UP_PULSES: usize = 4;
const HOLD_UP_LENGTH: usize = HOLD_UP_PULSES * 2 + 1;
const HOLD_UP_SHIFT: usize = HOLD_UP_PULSES + 1;
const THRESHOLD: u64 = 2;

const DIGITS: [u8; 16] = [
    0x3f, 0x06, 0x5b, 0x4f, //
    0x66, 0x6d, 0x7d, 0x07, //
    0x7f, 0x6f, 0x77, 0x7c, //
    0x39, 0x5e, 0x79, 0x71, //
];

#[entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

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

    // The delay object lets us wait for specified amounts of time (in milliseconds)
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let timer = timer::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

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

    let mut tm = TM1637::new(&mut clk, &mut dio, &mut delay);
    tm.init().unwrap(); // append `.unwrap()` to catch and handle exceptions in cost of extra ROM size
    tm.clear().unwrap();
    tm.set_brightness(0xff).unwrap();

    let mut update_display = |value: usize, holded: usize| {
        let mut digits = [0u8; 4];

        let d0 = (value / 1000).to_be_bytes()[3];
        let d1 = ((value % 1000) / 100).to_be_bytes()[3];
        let d2 = ((value % 100) / 10).to_be_bytes()[3];
        let d3 = (value % 10).to_be_bytes()[3];

        let mut is_leading_zero = true;

        for (i, d) in [d0, d1, d2, d3].iter().enumerate() {
            if *d != 0u8 && is_leading_zero {
                is_leading_zero = false;
            }
            // Remove leading zeros, except the last digit
            let code = if i != 3 && *d == 0u8 && is_leading_zero {
                0
            } else {
                DIGITS[*d as usize]
            };

            // If holded is greater than i, set the dot
            // e.g. holded = 3, then will be dots in each position
            digits[i] = if i >= holded { code } else { code | 0x80 };
        }

        tm.print_raw(0, &digits).unwrap();
    };

    let mut trigger_pin = pins.gp0.into_push_pull_output();
    let mut echo_pin = pins.gp1.into_pull_down_input();

    let mut last_start = 0;
    let mut last_end = 0;

    let mut is_trigger_high = false;
    let mut is_last_echo_high = false;

    let mut distances: [u64; HOLD_UP_LENGTH] = [0u64; HOLD_UP_LENGTH];
    let mut i: usize = 0;

    let mut total: usize = 0;

    let mut holded: usize = 0;

    let mut is_counted = false;

    loop {
        let now = timer.get_counter().ticks();

        if (now - last_start) > PULSE_INTERVAL {
            trigger_pin.set_high().unwrap();
            is_trigger_high = true;
            last_start = now;
        }

        if (now - last_start) > PULSE_LENGTH && is_trigger_high {
            trigger_pin.set_low().unwrap();
            is_trigger_high = false;
            last_end = now;
        }

        if is_last_echo_high && echo_pin.is_low().unwrap() {
            // 58 is the speed of sound in cm per microsecond
            let distance = (now - last_end) / 58;

            distances[i] = distance;

            let j = if i >= HOLD_UP_SHIFT {
                i - HOLD_UP_SHIFT
            } else {
                HOLD_UP_LENGTH + i - HOLD_UP_SHIFT
            };

            if distances[i] > 0 {
                let ratio = distances[j] / distances[i];

                if ratio > THRESHOLD {
                    holded += 1;
                    update_display(total, holded);
                } else {
                    if holded > 0 {
                        holded -= 1;
                    } else {
                        is_counted = false;
                    }
                    update_display(total, holded);
                }
            }

            if holded >= HOLD_UP_PULSES && !is_counted {
                total += 1;
                is_counted = true;
                update_display(total, holded);
            }

            i = if i + 1 == HOLD_UP_LENGTH { 0 } else { i + 1 };
        }

        is_last_echo_high = echo_pin.is_high().unwrap();
    }
}
