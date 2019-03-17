#![feature(await_macro, async_await, futures_api)]
extern crate ds2480b;

#[macro_use]
extern crate tokio;

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyUSB0";
#[cfg(windows)]
const DEFAULT_TTY: &str = "COM2";

use tokio::prelude::*;
use tokio_serial::SerialPort;

use ds2480b::DS2480B;
use hex::encode;
use std::time::Duration;

async fn scan<P: SerialPort + AsyncReadExt + AsyncWriteExt>(ds2480b: &mut DS2480B<P>) -> std::io::Result<()> {
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
                    return Ok(());
                },
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::InvalidData => {
                            eprintln!("{}", err);
                            return Err(err);
                        },
                        std::io::ErrorKind::TimedOut => {
                            return Err(err);
                        },
                        kind => panic!("{:?}: {}", kind, err),
                    }

                }
            }
        }
    }
}

fn main() {
    let mut args = std::env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());

    loop {
        let tty_path = tty_path.clone();
        tokio::run_async(async move {
            await!(tokio_timer::sleep(Duration::from_millis(100))).unwrap();
            let settings = tokio_serial::SerialPortSettings::default();
            let port = tokio_serial::Serial::from_path(&tty_path, &settings).unwrap();
            #[cfg(unix)]
            port.set_exclusive(false)
                .expect("Unable to set serial port exlusive");

            let mut ds2480b = DS2480B::new(port).expect("Failed opening serial port");
            loop {
                if let Err(_) = await!(scan(&mut ds2480b)) {
                    if let Err(err) = await!(ds2480b.detect()) {
                        eprintln!("{:?}, reconnect", err);
                        break;
                    }
                }
                await!(tokio_timer::sleep(Duration::from_millis(100))).unwrap();
            }
        });
    }
}
