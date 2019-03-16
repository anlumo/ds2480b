#![feature(await_macro, async_await, futures_api)]
#![allow(dead_code, unreachable_code, unused_attributes, unused_variables, unused_imports)]
#[macro_use]
extern crate tokio;

use tokio::prelude::*;

use tokio_serial::{Result, SerialPort, SerialPortSettings};
use tokio_timer::sleep;

use std::future::Future;
use std::time::Duration;

enum DS2480BCommand {
    Reset =                 0xC1,
    Pullup =                0x3B,
    DataMode =              0xE1,
    CommandMode =           0xE3,
    SearchAccelleratorOn =  0xB1,
    SearchAccelleratorOff = 0xA1,
    ConvertT =              0x44,
    PullupArm =             0xEF,
    PullupDisarm =          0xED,
    PulseTerminate =        0xF1,
    ReadScratchpad =        0xBE,
    SkipROM =               0xCC,
    MatchROM =              0x55,
    SearchROM =             0xF0,
}

pub enum DS2480BSearchMode {
    ROM,
    Alarm,
}

pub struct DS2480B<P: SerialPort + AsyncReadExt + AsyncWriteExt> {
    port: P,
}

impl<P: SerialPort + AsyncReadExt + AsyncWriteExt> DS2480B<P> {
    pub fn new(mut port: P) -> Result<Self> {
        port.set_all(&SerialPortSettings {
            baud_rate: 9600,
            data_bits: tokio_serial::DataBits::Eight,
            flow_control: tokio_serial::FlowControl::None,
            parity: tokio_serial::Parity::None,
            stop_bits: tokio_serial::StopBits::One,
            timeout: Duration::from_millis(100),
        })?;
        Ok(DS2480B { port })
    }

    async fn send_break(&mut self) -> Result<()> {
        if let Err(err) = self.port.set_baud_rate(2400) {
            Err(err)
        } else {
            await!(self.port.write_all_async(&[0u8]))?;
            self.port.set_baud_rate(9600)?;
            Ok(())
        }
    }

    async fn set_mode(&mut self, mode: DS2480BCommand) -> Result<()> {
        let bytes = [mode as u8];
        await!(self.port.write_all_async(&bytes))?;
        Ok(())
    }

    async fn send_command(&mut self, command: DS2480BCommand) -> Result<u8> {
        let bytes = [command as u8];
        await!(self.port.write_all_async(&bytes))?;

        let mut buf = [0u8];
        await!(self.port.read_exact_async(&mut buf))?;
        Ok(buf[0])
    }

    pub async fn reset(&mut self) -> Result<()> {
        loop {
            match await!(self.send_command(DS2480BCommand::Reset)) {
                Ok(0xCD) => {
                    await!(sleep(Duration::from_millis(2))).unwrap();
                    return Ok(());
                },
                Err(e) => return Err(e),
                _ => {
                    await!(self.send_break())?;
                    await!(sleep(Duration::from_millis(2))).unwrap();
                },
            }
        }
    }

    pub async fn search(&mut self, mode: DS2480BSearchMode) -> Result<()> {
        // let hdb = 0;
        // let last_hdb = 0;
        // let index = 0;

        await!(self.reset())?;
        await!(self.set_mode(DS2480BCommand::DataMode))?;
        await!(self.send_command(match mode {
            DS2480BSearchMode::ROM => DS2480BCommand::SearchROM,
            DS2480BSearchMode::Alarm => DS2480BCommand::SearchROM,
        }))?;
        await!(self.set_mode(DS2480BCommand::CommandMode))?;
        await!(self.set_mode(DS2480BCommand::SearchAccelleratorOn))?;
        await!(self.set_mode(DS2480BCommand::DataMode))?;

        // first search data
        let bytes = [0u8; 16];
        await!(self.port.write_all_async(&bytes))?;

        // read found ROM code
        let mut code = [0u8; 16];
        await!(self.port.read_exact_async(&mut code))?;
        await!(self.set_mode(DS2480BCommand::CommandMode))?;
        await!(self.set_mode(DS2480BCommand::SearchAccelleratorOff))?;

        await!(self.reset())?;


        Ok(())
    }

}
