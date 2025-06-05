use core::convert::TryInto;

use std::{sync::{Arc, Mutex}, thread};

use embedded_svc::{
    http::{Method}, // Headers
    io::{Write}, // Read
    wifi::{AuthMethod, ClientConfiguration, Configuration}
};

use esp_idf_svc::{
    eventloop::{EspEventLoop, EspSystemEventLoop, System}, 
    hal::{
        i2c::{I2cConfig, I2cDriver},
        prelude::Peripherals, units::FromValueType,
        delay
    }, 
    http::server::EspHttpServer, 
    log::EspLogger, 
    nvs::EspDefaultNvsPartition, 
    sys, 
    wifi::{BlockingWifi, EspWifi},
};

use log::info;

// use serde::Deserialize;
use bme280::{i2c::BME280};

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");

static INDEX_HTML: &str = include_str!("index.html");

const MAX_LEN: usize = 504;

const STACK_SIZE: usize = 10240;

const CHANNEL: u8 = 11;

#[derive(Debug, Clone)]
struct Measurements {
    temperature: f32,
    pressure: f32,
    humidity: f32,
}

fn main() -> anyhow::Result<()> {

    // --- GENERAL --- //
    sys::link_patches();
    EspLogger::initialize_default();

    info!("SSID: {WIFI_SSID:?}");
    info!("PASS: {WIFI_PASS:?}");

    let sys_loop: EspEventLoop<System> = EspSystemEventLoop::take()?;
    let nvs: EspDefaultNvsPartition = EspDefaultNvsPartition::take()?;
    let peripherals: Peripherals = Peripherals::take()?;

    let measurements = Arc::new(Mutex::new(Measurements {
        temperature: 0.0,
        pressure: 0.0,
        humidity: 0.0
    }));

    let m_http = measurements.clone();
    let m_bme280 = measurements.clone();
    // --- --- --- //

    // --- BME280 --- //
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    let config = I2cConfig::new().baudrate(400_u32.kHz().into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;
    let mut delay = delay::FreeRtos;
    let mut bme280 = BME280::new_primary(i2c);
    bme280.init(&mut delay).unwrap();
    // --- --- --- //

    // --- WIFI --- //
    let mut wifi: BlockingWifi<EspWifi<'_>> = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;
    // --- --- --- //

    // --- HTTP SERVER --- //
    let mut server = create_server()?;

    server.fn_handler("/", Method::Get, move |req| {

        let data = m_http.lock().unwrap();

        let html = INDEX_HTML
            .replace("{{temperature}}", &format!("{:.1}", &data.temperature))
            .replace("{{pressure}}", &format!("{:.0}", &data.pressure))
            .replace("{{humidity}}", &format!("{:.1}", &data.humidity));

        req.into_ok_response()?
            .write_all(html.as_bytes())
            .map(|_| ())
    })?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {ip_info:?}");

    thread::Builder::new()
        .name("BME280".into())
        .stack_size(16 * 1024)
        .spawn(move || {
            loop {
                match bme280.measure(&mut delay) {
                    Ok(measurements) => {
                        log::info!("Temperature = {:.1}°C", measurements.temperature);
                        log::info!("Pressure = {:.0} Pa", measurements.pressure);
                        log::info!("Relative Humidity = {:.1}%", measurements.humidity);

                        let mut data = m_bme280.lock().unwrap();
                        
                        data.temperature = measurements.temperature;
                        data.pressure = measurements.pressure;
                        data.humidity = measurements.humidity;
                    }

                    Err(e) => log::error!("Failed to get measurements: {:?}", e),
                }

                delay::FreeRtos::delay_ms(1000_u32);
            }
        })?;


    core::mem::forget(wifi);
    core::mem::forget(server);

    Ok(())
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {

    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration { 
        ssid: WIFI_SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal, 
        password: WIFI_PASS.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(())
}

fn create_server() -> anyhow::Result<EspHttpServer<'static>> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    Ok(EspHttpServer::new(&server_configuration)?)
}