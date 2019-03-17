#![feature(await_macro, async_await, futures_api, copy_within)]
#![allow(dead_code, unreachable_code, unused_attributes, unused_variables, unused_imports)]
#[macro_use]
extern crate tokio;

use tokio::prelude::*;

use tokio_serial::{Result, SerialPort, SerialPortSettings};
use tokio_timer::sleep;
use tokio_io_timeout::TimeoutStream;

use std::future::Future;
use std::time::Duration;

use bigwise::{Bw64, Bw128, Bigwise};

pub mod codes;
pub mod search;

pub struct DS2480B<P: SerialPort + AsyncReadExt + AsyncWriteExt> {
    stream: TimeoutStream<P>,
    level: codes::Level,
    mode: codes::Mode,
}

impl<P: SerialPort + AsyncReadExt + AsyncWriteExt> DS2480B<P> {
    pub fn new(mut port: P) -> Result<Self> {
        port.set_all(&SerialPortSettings {
            baud_rate: 9600,
            data_bits: tokio_serial::DataBits::Eight,
            flow_control: tokio_serial::FlowControl::None,
            parity: tokio_serial::Parity::None,
            stop_bits: tokio_serial::StopBits::One,
            timeout: Duration::from_millis(0), // this is ignored by tokio_serial!
        })?;
        let mut stream = TimeoutStream::new(port);
        stream.set_read_timeout(Some(Duration::from_millis(200)));
        stream.set_write_timeout(Some(Duration::from_millis(200)));
        Ok(DS2480B {
            stream,
            level: codes::Level::Normal,
            mode: codes::Mode::Command,
        })
    }

    async fn delay(&self) {
        await!(tokio_timer::sleep(Duration::from_millis(2))).unwrap();
    }
    async fn flush<'a>(&'a mut self) -> std::io::Result<()> {
        await!(self.stream.flush_async())
    }

    async fn write<'a>(&'a mut self, buffer: &'a [u8]) -> std::io::Result<()> {
        await!(self.flush())?;
        await!(self.stream.write_all_async(buffer))
    }

    async fn read<'a>(&'a mut self, mut buffer: &'a mut [u8]) -> std::io::Result<()> {
        eprintln!("Waiting for {} bytes...", buffer.len());
        await!(self.stream.read_exact_async(&mut buffer))?;
        // scan for unsolicited device notification
        if self.mode == codes::Mode::Command {
            let mut i = 0;
            while i < buffer.len() {
                if (buffer[i] & 0x2) == 0x1 {
                    eprintln!("Device notification!");
                    let mut fill = [0u8];
                    await!(self.stream.read_exact_async(&mut fill))?;
                    if i < buffer.len()-1 {
                        buffer.copy_within(i+1..buffer.len(), i);
                    }
                    buffer[buffer.len()-1] = fill[0];
                } else {
                    i += 1;
                }
            }
        }

        Ok(())
    }

    /// Reset all of the devices on the 1-Wire Net and return the result.
    ///
    /// Returns: true:  presense pulse(s) detected, device(s) reset
    ///          false: no presense pulses detected
    pub async fn reset(&mut self) -> Result<bool> {
        await!(self.level(codes::Level::Normal))?;

        let mut send_packet = Vec::new();
        if self.mode != codes::Mode::Command {
            self.mode = codes::Mode::Command;
            send_packet.push(codes::Command::CommandMode as u8);
        }
        send_packet.push((codes::Command::Comm as u8) | (codes::FunctionSelect::Reset as u8));

        await!(self.write(&send_packet))?;

        let mut buf = [0u8];
        await!(self.stream.read_exact_async(&mut buf))?;

        if (buf[0] & codes::reset_byte::RESET_MASK) == codes::reset_byte::PRESENCE || (buf[0] & codes::reset_byte::RESET_MASK) == codes::reset_byte::ALARMPRESENCE {
            Ok(true)
        } else {
            if (buf[0] & codes::reset_byte::CHIPID_MASK) != 0xc || (buf[0] >> 6) != 0x3 {
                eprintln!("Reset failed (read {:02x}), trying to find device again", buf[0]);
                await!(self.detect())?;
            }
            Ok(false)
        }
    }

    /// Attempt to resyc and detect a DS2480B and set the FLEX parameters
    ///
    /// Returns:  true  - DS2480B detected successfully
    ///           false - Could not detect DS2480B
    pub async fn detect(&mut self) -> Result<bool> {
        self.mode = codes::Mode::Command;

        // Send break. The tokio-serial API doesn't support sending native breaks, so we have to fake it
        eprintln!("Sending break...");
        await!(self.flush())?;
        self.stream.get_mut().set_baud_rate(2400)?;
        await!(self.write(&[0u8;3]))?;
        await!(self.flush())?;
        await!(self.delay());
        self.stream.get_mut().set_baud_rate(9600)?;
        await!(self.delay());

        let send_packet = [codes::Command::Reset as u8];
        await!(self.write(&send_packet))?;

        await!(self.delay());

        let send_packet = [
            // default PDSRC = 1.37Vus
            (codes::Command::Config as u8) | (codes::ParameterSelect::Slew as u8) | (codes::SlewRate::Slew1p37Vus as u8),
            // default W1LT = 10us
            (codes::Command::Config as u8) | (codes::ParameterSelect::Write1Low as u8) | (codes::Write1LowTime::Write10us as u8),
            // default DSO/WORT = 8us
            (codes::Command::Config as u8) | (codes::ParameterSelect::SampleOffset as u8) | (codes::SampleOffset::SampOff8us as u8),
            // construct the command to read the baud rate (to test command block)
            (codes::Command::Config as u8) | (codes::ParameterSelect::ParmRead as u8) | (codes::ParameterSelect::Baudrate as u8 >> 3),
            // also do 1 bit operation (to test 1-Wire block)
            (codes::Command::Comm as u8) | (codes::FunctionSelect::Bit as u8) | (codes::BitPolarity::One as u8),
        ];

        await!(self.write(&send_packet))?;

        let mut buf = [0u8; 5];
        await!(self.read(&mut buf))?;
        Ok((buf[3] & 0xF1) == 0x00 && (buf[3] & 0x0E) == 0x00 && (buf[4] & 0xF0) == 0x90 && (buf[4] & 0x0C) == 0x00)
    }

    /// Set the 1-Wire Net line level.
    ///
    /// Returns:  current 1-Wire Net level
    pub async fn level(&mut self, new_level: codes::Level) -> Result<codes::Level> {
        if new_level != self.level {
            let mut reset = false;
            let mut send_packet = Vec::new();
            if self.mode == codes::Mode::Command {
                self.mode = codes::Mode::Command;
                send_packet.push(codes::Command::CommandMode as u8);
            }
            if new_level == codes::Level::Normal {
                send_packet.push(codes::Command::PulseTerminate as u8);
                send_packet.push((codes::Command::Comm as u8) | (codes::FunctionSelect::Chmod as u8) | (codes::SpeedSelect::Pulse as u8));
                send_packet.push(codes::Command::PulseTerminate as u8);
                await!(self.write(&send_packet))?;

                let mut read_buffer = [0u8;2];
                await!(self.read(&mut read_buffer))?;
                if (read_buffer[0] & 0xE0) == 0xE0 && (read_buffer[1] & 0xE0) == 0xE0 {
                    reset = true;
                    self.level = codes::Level::Normal;
                }
            } else {
                send_packet.push((codes::Command::Config as u8) | (codes::ParameterSelect::Pulse5V as u8) | (codes::PulseTime::PulseInfinite as u8));
                send_packet.push((codes::Command::Comm as u8) | (codes::FunctionSelect::Chmod as u8) | (codes::SpeedSelect::Pulse as u8));

                await!(self.write(&send_packet))?;
                let mut read_buffer = [0u8;1];
                await!(self.read(&mut read_buffer))?;
                if (read_buffer[0] & 0x81) == 0 {
                    self.level = new_level;
                    reset = true;
                }
            }

            if reset != true {
                await!(self.detect())?;
            }
        }
        Ok(self.level)
    }

    /// The 'search' function returns a general search object.
    /// This function contains one parameter 'alarm_only'.
    /// When 'alarm_only' is true the find alarm command
    /// 0xEC is sent instead of the normal search command 0xF0.
    /// Using the find alarm command 0xEC will limit the search to only
    /// 1-Wire devices that are in an 'alarm' state.
    pub fn search(&mut self, alarm_only: bool) -> search::Search<P> {
        search::Search::new(self, alarm_only)
    }
}
