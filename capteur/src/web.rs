use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0, RTC};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::rtc::{DateTime, Rtc};
use rand::RngCore;
use static_cell::StaticCell;
use dotenvy_macro::*;

use core::fmt::write;
use embassy_net::dns::DnsSocket;

use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::request::{Method, RequestBuilder};

use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_time::Timer;
use embedded_nal_async::{Dns, TcpConnect};
use heapless::String;

use crate::{Measure, MEASURE_SIGNAL, NETWORK_STACK_SIGNAL};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = dotenv!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = dotenv!("WIFI_PASSWORD");

#[embassy_executor::task]
pub async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

pub struct NetworkPeriphals {
    pub pin23: PIN_23,
    pub pin24: PIN_24,
    pub pin25: PIN_25,
    pub pin29: PIN_29,
    pub pio: PIO0,
    pub dma: DMA_CH0,
    pub rtc: RTC,
}

#[embassy_executor::task]
pub async fn network_stack(spawner: Spawner, p: NetworkPeriphals) {
    let mut rng = RoscRng;
    let mut rtc = Rtc::new(p.rtc);

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.pin23, Level::Low);
    let cs = Output::new(p.pin25, Level::High);
    let mut pio = Pio::new(p.pio, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.pin24,
        p.pin29,
        p.dma,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    loop {
        match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    info!("waiting for link up...");
    while !stack.is_link_up() {
        Timer::after_millis(500).await;
    }
    info!("Link is up!");

    info!("waiting for stack to be up...");
    stack.wait_config_up().await;
    info!("Stack is up!");

    let mut tls_read_buffer = [0; 16640];
    let mut tls_write_buffer = [0; 16640];

    let client_state = TcpClientState::<1, 1024, 1024>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let dns_client = DnsSocket::new(stack);
    let tls_config: TlsConfig<'_> = TlsConfig::new(
        seed,
        &mut tls_read_buffer,
        &mut tls_write_buffer,
        TlsVerify::None,
    );

    let mut http_client = HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);

    init_rtc(&mut http_client, &mut rtc).await;

    NETWORK_STACK_SIGNAL.signal(true);

    loop {
        let now = match rtc.now() {
            Ok(now) => now,
            Err(_) => {
                error!("RTC is not running");
                continue;
            }
        };
        let measure = MEASURE_SIGNAL.wait().await;
        post_measure(&mut http_client, measure, now).await;
    }
}

async fn init_rtc<'a, T, U>(http_client: &mut HttpClient<'a, T, U>, rtc: &mut Rtc<'a, RTC>)
where
    T: TcpConnect + 'a,
    U: Dns + 'a,
{
    rtc.set_datetime(DateTime {
        year: 2024,
        month: 11,
        day: 14,
        day_of_week: embassy_rp::rtc::DayOfWeek::Thursday,
        hour: 20,
        minute: 0,
        second: 0,
    })
    .unwrap();
}

async fn post_measure<'a, T, U>(
    http_client: &mut HttpClient<'a, T, U>,
    measure: Measure,
    now: DateTime,
) where
    T: TcpConnect + 'a,
    U: Dns + 'a,
{
    let url = "http://192.168.1.31:3000/measure";

    info!("connecting to {}", &url);

    let mut body: String<100> = String::new();
    body.push('{').unwrap();
    write(
        &mut body,
        format_args!(
            "\"timestamp\": \"{}-{}-{}T{}:{}:{}Z\",",
            now.year, now.month, now.day, now.hour, now.minute, now.second
        ),
    )
    .unwrap();
    write(
        &mut body,
        format_args!("\"temperature\": {},", measure.temperature),
    )
    .unwrap();
    write(
        &mut body,
        format_args!("\"humidity\": {}", measure.humidity),
    )
    .unwrap();
    body.push('}').unwrap();

    let mut request = http_client
        .request(Method::POST, &url)
        .await
        .unwrap()
        .body(body.as_bytes())
        .content_type(reqwless::headers::ContentType::ApplicationJson);
    let mut rx_buffer = [0; 8192];
    match request.send(&mut rx_buffer).await {
        _ => {}
    };
}
