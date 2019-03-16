#![feature(await_macro, async_await, futures_api)]
extern crate ds2480b;

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyUSB0";
#[cfg(windows)]
const DEFAULT_TTY: &str = "COM2";

use ds2480b::DS2480B;
use hex::encode;

fn main() {
    let mut args = std::env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());

    let settings = tokio_serial::SerialPortSettings::default();
    let port = tokio_serial::Serial::from_path(tty_path, &settings).unwrap();
    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("Unable to set serial port exlusive");


    tokio::run_async(async move {
        let mut ds2480b = DS2480B::new(port).expect("Failed opening serial port");
        loop {
            let mut search = ds2480b.search(false);
            eprintln!("Searching...");
            loop {
                match await!(search.next()) {
                    Ok(Some((device, new_search))) => {
                        eprintln!("Found Device {}", encode(device));
                        search = new_search;
                    },
                    Ok(None) => {
                        eprintln!("Search done.");
                        break;
                    },
                    Err(err) => {
                        panic!("{}", err);
                    }
                }
            }
        }
    });
}
