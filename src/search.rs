use std::io::{Result, Error};
use tokio::prelude::*;
use tokio_serial::SerialPort;
use bigwise::{Bw64, Bw128, Bigwise};
use super::{DS2480B, codes};

pub struct Search<'a, P: SerialPort + AsyncReadExt + AsyncWriteExt> {
    device: &'a mut DS2480B<P>,
    alarm_only: bool,
    last_discrepancy: u32,
    last_family_discrepancy: u32,
    last_device: bool,
}

/// The 'search' struct does a general search.
///
/// Usage: call next() over and over again until it returns Ok(None)
/// the u8 arrays yielded are the device IDs
/// Note that you can't reuse your old search object due to the
/// borrow checker. Use the object returned in the tuple instead
impl<'a, P: SerialPort + AsyncReadExt + AsyncWriteExt> Search<'a, P> {
    pub fn new(device: &'a mut DS2480B<P>, alarm_only: bool) -> Self {
        Search {
            device,
            alarm_only,
            last_discrepancy: 0,
            last_family_discrepancy: 0,
            last_device: false,
        }
    }

    pub async fn next(mut self) -> Result<Option<([u8;8], Search<'a, P>)>> {
        if self.last_device {
            return Ok(None);
        }
        // if there are no parts on 1-wire, returns false
        if !await!(self.device.reset())? {
            return Ok(None);
        }

        let mut send_packet = Vec::new();
        if self.device.mode != codes::Mode::Data {
            send_packet.push(codes::Command::DataMode as u8);
        }

        if self.alarm_only {
            send_packet.push(codes::Command::SearchAlarm as u8);
        } else {
            send_packet.push(codes::Command::SearchROM as u8);
        }

        send_packet.push(codes::Command::CommandMode as u8);
        send_packet.push((codes::Command::Comm as u8) | (codes::FunctionSelect::SearchOn as u8));

        send_packet.push(codes::Command::DataMode as u8);

        let rom = Bw64::empty();

        let mut last_zero = 0;
        let mut search_id = Bw128::empty();

        // only modify bits if not the first search
        if self.last_discrepancy != 0 {
            for i in 0..64 {
                if i < self.last_discrepancy-1 {
                    // before last discrepancy
                    search_id.set(i*2+1, rom.get(i));
                } else if i == self.last_discrepancy-1 {
                    // at last discrepancy
                    search_id.set(i*2+1, true);
                }
                // after last discrepancy so leave zeros
            }
        }
        send_packet.extend_from_slice(&search_id.to_bytes());

        send_packet.push(codes::Command::CommandMode as u8);
        send_packet.push((codes::Command::Comm as u8) | (codes::FunctionSelect::SearchOff as u8));

        self.device.mode = codes::Mode::Command;
        await!(self.device.write(&send_packet))?;

        let mut buf = [0u8;17];
        await!(self.device.read(&mut buf))?;

        // no idea why swapping is necessary here
        for i in 0..8 {
            buf.swap(i*2+1, i*2+2);
        }
        search_id = Bw128::from_bytes(&buf[1..]);
        let mut tmp_rom = Bw64::empty();
        // interpret the bit stream
        for i in 0..64 {
            // get the ROM bit
            tmp_rom.set(i, search_id.get(i*2+1));

            // check LastDiscrepancy
            if search_id.get(i*2) && !search_id.get(i*2+1) {
                last_zero = i+1;
                // check last_family_discrepancy
                if i < 8 {
                    self.last_family_discrepancy = i+1;
                }
            }
        }

        let mut crc8 = codes::CRC8::new();
        let tmp_rom_bytes = tmp_rom.to_bytes();
        for byte in tmp_rom_bytes.iter() {
            crc8.calc(*byte);
        }

        if crc8.0 != 0 || self.last_discrepancy == 63 || tmp_rom_bytes[0] == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "CRC8 failed"));
        }
        self.last_discrepancy = last_zero;
        if self.last_discrepancy == 0 {
            self.last_device = true;
        }
        let mut rom = [0u8;8];
        rom.copy_from_slice(&tmp_rom_bytes);
        Ok(Some((rom, self)))
    }
}
