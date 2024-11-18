#![no_std]
#![no_main]

pub mod capteur;
pub mod web;

use crate::capteur::measure_task;

use defmt::*;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use web::{network_stack, NetworkPeriphals};

use {defmt_rtt as _, panic_probe as _};

pub struct Measure {
    pub temperature: f64,
    pub humidity: f64,
}

pub static NETWORK_STACK_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static MEASURE_SIGNAL: Signal<CriticalSectionRawMutex, Measure> = Signal::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    unwrap!(spawner.spawn(measure_task(p.PIN_21)));

    let network_peripherals = NetworkPeriphals {
        pin23: p.PIN_23,
        pin24: p.PIN_24,
        pin25: p.PIN_25,
        pin29: p.PIN_29,
        pio: p.PIO0,
        dma: p.DMA_CH0,
        rtc: p.RTC,
    };
    unwrap!(spawner.spawn(network_stack(spawner, network_peripherals)));
}
