use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration as WifiConfiguration};
use esp_idf_hal::{peripheral::Peripheral, prelude::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::{EspDefaultNvsPartition, EspNvsPartition, NvsDefault},
    ping::Configuration as PingConfiguration,
    ping::EspPing,
    timer::{EspTaskTimerService, EspTimerService, Task},
    wifi::{AsyncWifi, EspWifi},
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use heapless::String;
use log::*;

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

pub fn wifi(
    modem: impl Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspNvsPartition<NvsDefault>>,
    timer_service: EspTimerService<Task>,
) -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {
    use futures::executor::block_on;
    let mut wifi = AsyncWifi::wrap(
        EspWifi::new(modem, sysloop.clone(), nvs)?,
        sysloop,
        timer_service.clone(),
    )?;

    block_on(connect_wifi(&mut wifi))?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    println!("Wifi DHCP info: {:?}", ip_info);

    EspPing::default().ping(ip_info.subnet.gateway, &PingConfiguration::default())?;
    Ok(wifi)
}

async fn connect_wifi(wifi: &mut AsyncWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: WifiConfiguration = WifiConfiguration::Client(ClientConfiguration {
        ssid: String::<32>::try_from(SSID).expect("SSID is too long"),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: String::<64>::try_from(PASS).expect("password is too long"),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start().await?;
    info!("Wifi started");

    wifi.connect().await?;
    info!("Wifi connected");

    wifi.wait_netif_up().await?;
    info!("Wifi netif up");

    Ok(())
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();
    let timer_service = EspTaskTimerService::new().unwrap();

    let _wifi = wifi(
        peripherals.modem,
        sysloop,
        Some(EspDefaultNvsPartition::take().unwrap()),
        timer_service,
    )
    .unwrap();
}
