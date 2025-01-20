use core::fmt::write;
use core::str::Utf8Error;
use defmt::*;
use dotenvy_macro::*;
use embassy_rp::peripherals::RTC;
use embassy_rp::rtc::{DateTime, DayOfWeek, Rtc, RtcError};
use embedded_nal_async::{Dns, TcpConnect};
use heapless::String;
use reqwless::{client::HttpClient, request::Method};
use serde::Deserialize;

const API_URL: &str = dotenv!("API_URL");

#[derive(Deserialize, Debug, Format)]
struct ApiResponse<'a> {
    now: &'a str,
    weekday: u8,
}

#[derive(Format, Debug)]
pub enum RTCInitError {
    APIError,
    InvalidAPIResponse,
    APIUnavailable,
    RtcError,
    DateTimeError,
}

impl From<reqwless::Error> for RTCInitError {
    fn from(value: reqwless::Error) -> Self {
        warn!("{}", value);
        Self::APIError
    }
}

impl From<core::fmt::Error> for RTCInitError {
    fn from(_: core::fmt::Error) -> Self {
        Self::APIError
    }
}

impl From<Utf8Error> for RTCInitError {
    fn from(_: Utf8Error) -> Self {
        Self::InvalidAPIResponse
    }
}

impl From<RtcError> for RTCInitError {
    fn from(_: RtcError) -> Self {
        Self::RtcError
    }
}

impl TryFrom<ApiResponse<'_>> for DateTime {
    type Error = RTCInitError;

    fn try_from(value: ApiResponse<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            year: value.now[0..4]
                .parse::<u16>()
                .map_err(|_| RTCInitError::DateTimeError)?,
            month: value.now[5..7]
                .parse::<u8>()
                .map_err(|_| RTCInitError::DateTimeError)?,
            day: value.now[8..10]
                .parse::<u8>()
                .map_err(|_| RTCInitError::DateTimeError)?,
            day_of_week: day_of_week_from_u8(value.weekday).unwrap_or(DayOfWeek::Monday),
            hour: value.now[11..13]
                .parse::<u8>()
                .map_err(|_| RTCInitError::DateTimeError)?,
            minute: value.now[14..16]
                .parse::<u8>()
                .map_err(|_| RTCInitError::DateTimeError)?,
            second: value.now[17..19]
                .parse::<u8>()
                .map_err(|_| RTCInitError::DateTimeError)?,
        })
    }
}

fn day_of_week_from_u8(v: u8) -> Result<DayOfWeek, ()> {
    Ok(match v {
        0 => DayOfWeek::Sunday,
        1 => DayOfWeek::Monday,
        2 => DayOfWeek::Tuesday,
        3 => DayOfWeek::Wednesday,
        4 => DayOfWeek::Thursday,
        5 => DayOfWeek::Friday,
        6 => DayOfWeek::Saturday,
        _ => return Err(()),
    })
}

pub async fn init_rtc<'a, T, U>(
    http_client: &mut HttpClient<'a, T, U>,
    rtc: &mut Rtc<'a, RTC>,
) -> Result<(), RTCInitError>
where
    T: TcpConnect + 'a,
    U: Dns + 'a,
{
    let mut url: String<100> = String::new();
    write(&mut url, format_args!("{API_URL}/now"))?;

    let mut request = match http_client.request(Method::GET, &url).await {
        Ok(request) => request,
        Err(err) => {
            warn!("Error when building the request, passing... {}", err);
            return Err(RTCInitError::APIError);
        }
    };

    let mut rx_buffer = [0; 8192];
    let body = request
        .send(&mut rx_buffer)
        .await?
        .body()
        .read_to_end()
        .await?;
    let response = match serde_json_core::de::from_slice::<ApiResponse>(body) {
        Ok((output, _)) => {
            info!("Api response: {}", output);
            output
        }
        Err(_) => {
            return Err(RTCInitError::InvalidAPIResponse);
        }
    };

    rtc.set_datetime(DateTime::try_from(response)?)?;

    Ok(())
}
