use core::ffi::c_char;
use core::ffi::c_int;
use core::slice;
use core::str;
use embedded_svc::http::Method::Get;
use embedded_svc::io::Write;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration as WifiConfiguration};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::{peripheral::Peripheral, prelude::Peripherals};
use esp_idf_svc::io::EspIOError;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::{Configuration as HttpConfiguration, EspHttpServer},
    nvs::{EspDefaultNvsPartition, EspNvsPartition, NvsDefault},
    ping::Configuration as PingConfiguration,
    ping::EspPing,
    timer::{EspTaskTimerService, EspTimerService, Task},
    wifi::{AsyncWifi, EspWifi},
};
use esp_idf_sys::{
    self as _, esp_vfs_fat_mount_config_t, esp_vfs_fat_spiflash_mount_ro,
    esp_vfs_fat_spiflash_mount_rw_wl, fclose, fgets, fopen, wl_handle_t, CONFIG_WL_SECTOR_SIZE,
    ESP_OK, WL_INVALID_HANDLE,
}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use heapless::String;
use log::*;
use std::ffi::CString;

macro_rules! env_with_default {
    ($var:literal, $default:expr) => {
        match option_env!($var) {
            Some(val) => val,
            None => $default,
        }
    };
}

const SSID: &str = env_with_default!("RUST_ESP32_STD_DEMO_WIFI_SSID", "MyDefaultSSID");
const PASS: &str = env_with_default!("RUST_ESP32_STD_DEMO_WIFI_PASS", "MyDefaultPass");

unsafe fn c_char_to_u8_slice(ptr: *const c_char) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }

    // Calculate length by scanning for the null terminator
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }

    // Reinterpret the pointer as *const u8 and create a slice
    slice::from_raw_parts(ptr, len)
}

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
    info!("Wifi started.");

    wifi.connect().await?;
    info!("Wifi connected.");

    wifi.wait_netif_up().await?;
    info!("Wifi netif up.");

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

    let base_path = CString::new("/webapp").unwrap(); // Make sure it has no interior nulls
    let partition_label = CString::new("webapp").unwrap();
    let config = esp_vfs_fat_mount_config_t {
        format_if_mount_failed: false,
        max_files: 4,
        allocation_unit_size: CONFIG_WL_SECTOR_SIZE as usize,
        disk_status_check_enable: false,
        use_one_fat: false,
    };
    let mut wl_handle: wl_handle_t = WL_INVALID_HANDLE;
    let result = unsafe {
        esp_vfs_fat_spiflash_mount_ro(
            base_path.as_ptr(),
            partition_label.as_ptr(),
            &config as *const _,
            // &mut wl_handle,
        )
    };

    match result {
        ESP_OK => {
            info!("Mount FAT partition successful.");
        }
        _ => {
            error!(
                "An error occurred mounting the FAT partition. Result code: {}",
                result
            );
        }
    }

    let server_config = HttpConfiguration {
        uri_match_wildcard: true,
        ..Default::default()
    };
    let mut server = EspHttpServer::new(&server_config).unwrap();

    server
        .fn_handler("/webapp/*", Get, move |req| {
            info!("Got URI: {}", req.uri());
            let fh = unsafe {
                fopen(
                    CString::new("/webapp/build.ts").unwrap().as_ptr(),
                    CString::new("r").unwrap().as_ptr(),
                )
            };
            if fh.is_null() {
                let mut resp =
                    req.into_response(404, Some("Not found."), &[("Content-Type", "text/html")])?;
                resp.write_all(b"No suitable file could be found.")?;
                return Ok::<(), EspIOError>(());
            }

            let mut resp = req.into_response(200, Some("OK"), &[("Content-Type", "text/html")])?;

            let mut buf = [0u8; 128];
            loop {
                let line =
                    unsafe { fgets(buf.as_mut_ptr() as *mut c_char, buf.len() as c_int, fh) };
                if line.is_null() {
                    break;
                }
                resp.write_all(unsafe { c_char_to_u8_slice(line) })?;
            }
            unsafe { fclose(fh) };

            Ok::<(), EspIOError>(())
        })
        .unwrap();

    loop {
        FreeRtos::delay_ms(2);
    }
}
