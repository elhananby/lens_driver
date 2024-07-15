use serde::de::DeserializeOwned;
use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LensError {
    #[error("Serial port error: {0}")]
    SerialPortError(#[from] serialport::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Handshake failed")]
    HandshakeFailed,
    #[error("CRC mismatch")]
    CrcMismatch,
    #[error("Unexpected response")]
    UnexpectedResponse,
    #[error("Invalid mode")]
    InvalidMode,
}

pub struct Lens {
    port: Box<dyn SerialPort>,
    debug: bool,
    firmware_type: char,
    firmware_version: (u8, u8, u16, u16),
    device_id: String,
    max_output_current: f32,
    mode: u8,
    lens_serial: String,
}

impl Lens {
    pub fn new(port_name: &str, debug: bool) -> Result<Self, LensError> {
        let port = serialport::new(port_name, 115_200)
            .timeout(Duration::from_secs(1))
            .open()?;

        let mut lens = Lens {
            port,
            debug,
            firmware_type: ' ',
            firmware_version: (0, 0, 0, 0),
            device_id: String::new(),
            max_output_current: 0.0,
            mode: 0,
            lens_serial: String::new(),
        };

        lens.port.write_all(b"Start")?;
        let mut buf = [0u8; 7];
        lens.port.read_exact(&mut buf)?;

        if &buf != b"Ready\r\n" {
            return Err(LensError::HandshakeFailed);
        }

        lens.firmware_type = lens.get_firmware_type()?;
        lens.firmware_version = lens.get_firmware_version()?;
        lens.device_id = lens.get_device_id()?;
        lens.max_output_current = lens.get_max_output_current()?;
        lens.set_temperature_limits(20.0, 40.0)?;
        lens.refresh_active_mode()?;
        lens.lens_serial = lens.get_lens_serial_number()?;

        if lens.debug {
            println!("=== Lens initialization complete ===");
        }

        Ok(lens)
    }

    fn send_command<T: DeserializeOwned>(&mut self, command: &[u8]) -> Result<T, LensError> {
        let crc = crc_16(command);
        let mut full_command = command.to_vec();
        full_command.extend_from_slice(&crc.to_le_bytes());

        if self.debug {
            println!("Sending: {:?}", full_command);
        }

        self.port.write_all(&full_command)?;

        let mut response = Vec::new();
        self.port.read_to_end(&mut response)?;

        if self.debug {
            println!("Received: {:?}", response);
        }

        if response.len() < 4 {
            return Err(LensError::UnexpectedResponse);
        }

        let data = &response[..response.len() - 4];
        let received_crc =
            u16::from_le_bytes([response[response.len() - 4], response[response.len() - 3]]);

        if crc_16(data) != received_crc {
            return Err(LensError::CrcMismatch);
        }

        Ok(serde_json::from_slice(data)?)
    }

    pub fn get_max_output_current(&mut self) -> Result<f32, LensError> {
        let response: i16 = self.send_command(b"CrMA\x00\x00")?;
        Ok(response as f32 / 100.0)
    }

    pub fn get_firmware_type(&mut self) -> Result<char, LensError> {
        let response: String = self.send_command(b"H")?;
        Ok(response.chars().next().unwrap_or(' '))
    }

    pub fn get_firmware_branch(&mut self) -> Result<u8, LensError> {
        self.send_command(b"F")
    }

    pub fn get_device_id(&mut self) -> Result<String, LensError> {
        self.send_command(b"IR\x00\x00\x00\x00\x00\x00\x00\x00")
    }

    pub fn get_firmware_version(&mut self) -> Result<(u8, u8, u16, u16), LensError> {
        self.send_command(b"V\x00")
    }

    pub fn get_lens_serial_number(&mut self) -> Result<String, LensError> {
        self.send_command(b"X")
    }

    pub fn eeprom_write_byte(&mut self, address: u8, byte: u8) -> Result<u8, LensError> {
        self.send_command(&[b'Z', b'w', address, byte])
    }

    pub fn eeprom_dump(&mut self) -> Result<Vec<u8>, LensError> {
        (0..=255)
            .map(|i| self.send_command(&[b'Z', b'r', i]))
            .collect()
    }

    pub fn get_temperature(&mut self) -> Result<f32, LensError> {
        let response: i16 = self.send_command(b"TCA")?;
        Ok(response as f32 * 0.0625)
    }

    pub fn set_temperature_limits(
        &mut self,
        lower: f32,
        upper: f32,
    ) -> Result<(u8, f32, f32), LensError> {
        let command = [
            b'P',
            b'w',
            b'T',
            b'A',
            ((upper * 16.0) as i16).to_le_bytes()[0],
            ((upper * 16.0) as i16).to_le_bytes()[1],
            ((lower * 16.0) as i16).to_le_bytes()[0],
            ((lower * 16.0) as i16).to_le_bytes()[1],
        ];
        let response: (u8, i16, i16) = self.send_command(&command)?;
        let (error, min_fp, max_fp) = response;
        let (min_fp, max_fp) = if self.firmware_type == 'A' {
            (min_fp as f32 / 200.0 - 5.0, max_fp as f32 / 200.0 - 5.0)
        } else {
            (min_fp as f32 / 200.0, max_fp as f32 / 200.0)
        };
        Ok((error, min_fp, max_fp))
    }

    pub fn get_current(&mut self) -> Result<f32, LensError> {
        let response: i16 = self.send_command(b"Ar\x00\x00")?;
        Ok(response as f32 * self.max_output_current / 4095.0)
    }

    pub fn set_current(&mut self, current: f32) -> Result<(), LensError> {
        if self.mode != 1 {
            return Err(LensError::InvalidMode);
        }
        let raw_current = (current * 4095.0 / self.max_output_current) as i16;
        let command = [
            b'A',
            b'w',
            raw_current.to_le_bytes()[0],
            raw_current.to_le_bytes()[1],
        ];
        self.send_command(&command)?;
        Ok(())
    }

    pub fn get_diopter(&mut self) -> Result<f32, LensError> {
        let raw_diopter: i16 = self.send_command(b"PrDA\x00\x00\x00\x00")?;
        Ok(if self.firmware_type == 'A' {
            raw_diopter as f32 / 200.0 - 5.0
        } else {
            raw_diopter as f32 / 200.0
        })
    }

    pub fn set_diopter(&mut self, diopter: f32) -> Result<(), LensError> {
        if self.mode != 5 {
            return Err(LensError::InvalidMode);
        }
        let raw_diopter = if self.firmware_type == 'A' {
            ((diopter + 5.0) * 200.0) as i16
        } else {
            (diopter * 200.0) as i16
        };
        let command = [
            b'P',
            b'w',
            b'D',
            b'A',
            raw_diopter.to_le_bytes()[0],
            raw_diopter.to_le_bytes()[1],
            0,
            0,
        ];
        self.send_command(&command)?;
        Ok(())
    }

    pub fn to_focal_power_mode(&mut self) -> Result<(f32, f32), LensError> {
        let (_error, max_fp_raw, min_fp_raw): (u8, i16, i16) = self.send_command(b"MwCA")?;
        let (mut min_fp, mut max_fp) = (min_fp_raw as f32 / 200.0, max_fp_raw as f32 / 200.0);
        if self.firmware_type == 'A' {
            min_fp -= 5.0;
            max_fp -= 5.0;
        }
        self.refresh_active_mode()?;
        Ok((min_fp, max_fp))
    }

    pub fn to_current_mode(&mut self) -> Result<(), LensError> {
        self.send_command(b"MwDA")?;
        self.refresh_active_mode()?;
        Ok(())
    }

    pub fn refresh_active_mode(&mut self) -> Result<u8, LensError> {
        self.mode = self.send_command(b"MMA")?;
        Ok(self.mode)
    }
}

fn crc_16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}
