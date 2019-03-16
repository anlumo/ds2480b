#![feature(await_macro, async_await, futures_api)]
extern crate ds2480b;

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyUSB0";
#[cfg(windows)]
const DEFAULT_TTY: &str = "COM1";

use ds2480b::{DS2480B, DS2480BSearchMode};

fn main() {
    let mut args = std::env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());

    let settings = tokio_serial::SerialPortSettings::default();
    let port = tokio_serial::Serial::from_path(tty_path, &settings).unwrap();
    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("Unable to set serial port exlusive");

    let mut ds2480b = DS2480B::new(port).expect("Failed opening serial port");

    tokio::run_async(async move {
        await!(ds2480b.search(DS2480BSearchMode::ROM)).expect("Failed scanning for devices");
    });
}
