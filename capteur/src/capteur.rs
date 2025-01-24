use am2301::{measure_once_timeout, Measure as _Measure};
use defmt::*;
use embassy_futures::join::join;
use embassy_rp::gpio::Flex;
use embassy_rp::peripherals::PIN_21;
use embassy_time::{Instant, Timer};

use crate::{Measure, MEASURE_SIGNAL, NETWORK_STACK_SIGNAL};

async fn wait_for_network_stack() {
    let mut stack_is_up = NETWORK_STACK_SIGNAL.wait().await;
    while !stack_is_up {
        stack_is_up = NETWORK_STACK_SIGNAL.wait().await;
    }
}

#[embassy_executor::task]
pub async fn measure_task(pin: PIN_21) -> ! {
    let mut pin = Flex::new(pin);

    // Wait for device to initialized
    join(Timer::after_secs(2), wait_for_network_stack()).await;

    loop {
        let start = Instant::now();
        match measure_once_timeout(&mut pin).await {
            Ok(_Measure {
                humidity,
                temperature,
            }) => {
                MEASURE_SIGNAL.signal(Measure {
                    temperature,
                    humidity,
                });
                // info!("Temperature = {} and humidity = {}", temperature, humidity);
            }
            Err(err) => {
                warn!("Error while measure temperature and humidity: {:?}", err)
            }
        }
        let delay = 5 - start.elapsed().as_secs();
        info!("Sleeping for {}s", delay);
        Timer::after_secs(delay).await;
    }
}
