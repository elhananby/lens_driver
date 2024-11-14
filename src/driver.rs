use serialport::{SerialPort, new};
use std::time::Duration;
use std::thread;
use log::{debug, info, error};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LensError {
    #[error("Serial port error: {0}")]
    SerialPort(#[from] serialport::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid mode")]
    InvalidMode,
    
    #[error("Handshake failed")]
    HandshakeFailed,
    
    #[error("CRC check failed")]
    CrcError,
    
    #[error("Wrong operation mode: expected {expected:?}, got {actual:?}")]
    WrongMode {
        expected: LensMode,
        actual: Option<LensMode>,
    },
}

pub type Result<T> = std::result::Result<T, LensError>;

#[derive(FromPrimitive, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum LensMode {
    Current = 1,
    FocalPower = 5,
}

pub struct LensDriver {
    port: Box<dyn SerialPort>,
    firmware_type: String,
    firmware_version: (u8, u8, u16, u16),
    max_output_current: f64,
    mode: Option<LensMode>,
}

impl LensDriver {
    pub fn new(port_name: &str, debug: bool) -> Result<Self> {
        if debug {
            env_logger::try_init().ok();
        }

        // let mut settings = SerialPortSettings::default();
        // settings.baud_rate = 115200;
        // settings.timeout = Duration::from_secs(1);

        let port = new(port_name, 115200).timeout(Duration::from_secs(1)).open()?;

        let mut driver = LensDriver {
            port,
            firmware_type: String::new(),
            firmware_version: (0, 0, 0, 0),
            max_output_current: 0.0,
            mode: None,
        };

        driver.handshake()?;
        driver.init()?;

        Ok(driver)
    }

    fn handshake(&mut self) -> Result<()> {
        debug!("Performing handshake");
        self.port.write_all(b"Start")?;

        let mut response = [0u8; 7];
        self.port.read_exact(&mut response)?;

        if &response != b"Ready\r\n" {
            return Err(LensError::HandshakeFailed);
        }

        debug!("Handshake successful");
        Ok(())
    }

    fn init(&mut self) -> Result<()> {
        self.firmware_type = self.get_firmware_type()?;
        self.firmware_version = self.get_firmware_version()?;
        self.max_output_current = self.get_max_output_current()?;
        self.refresh_active_mode()?;
        Ok(())
    }

    /// Get the current mode
    pub fn mode(&self) -> Option<LensMode> {
        self.mode
    }

    /// Get the firmware type
    pub fn firmware_type(&self) -> &str {
        &self.firmware_type
    }

    /// Get the firmware version
    pub fn firmware_version(&self) -> (u8, u8, u16, u16) {
        self.firmware_version
    }

    /// Get the maximum output current
    pub fn max_output_current(&self) -> f64 {
        self.max_output_current
    }
    
    fn get_firmware_type(&mut self) -> Result<String> {
        debug!("Getting firmware type");
        let response = self.send_command(b"H", 1)?;
        Ok(String::from_utf8_lossy(&response).to_string())
    }

    fn get_firmware_version(&mut self) -> Result<(u8, u8, u16, u16)> {
        debug!("Getting firmware version");
        let response = self.send_command(b"V\x00", 6)?;
        Ok((
            response[0],
            response[1],
            u16::from_be_bytes([response[2], response[3]]),
            u16::from_be_bytes([response[4], response[5]]),
        ))
    }

    fn get_max_output_current(&mut self) -> Result<f64> {
        debug!("Getting maximum output current");
        let response = self.send_command(b"CrMA\x00\x00", 2)?;
        let max_current = i16::from_be_bytes([response[0], response[1]]) as f64 / 100.0;
        debug!("Maximum output current: {} mA", max_current);
        Ok(max_current)
    }

    pub fn get_temperature(&mut self) -> Result<f64> {
        debug!("Getting temperature");
        let response = self.send_command(b"TCA", 2)?;
        let temp = i16::from_be_bytes([response[0], response[1]]) as f64 * 0.0625;
        debug!("Temperature: {}°C", temp);
        Ok(temp)
    }

    pub fn set_mode(&mut self, mode: LensMode) -> Result<Option<(f64, f64)>> {
        info!("Setting mode to {:?}", mode);
        match mode {
            LensMode::Current => {
                self.send_command(b"MwDA", 0)?;
                self.mode = Some(LensMode::Current);
                Ok(None)
            }
            LensMode::FocalPower => {
                let response = self.send_command(b"MwCA", 5)?;
                self.mode = Some(LensMode::FocalPower);
                
                let min_fp_raw = i16::from_be_bytes([response[3], response[4]]) as f64 / 200.0;
                let max_fp_raw = i16::from_be_bytes([response[1], response[2]]) as f64 / 200.0;
                
                let (min_fp, max_fp) = if self.firmware_type == "A" {
                    (min_fp_raw - 5.0, max_fp_raw - 5.0)
                } else {
                    (min_fp_raw, max_fp_raw)
                };
                
                debug!("Focal power range: {} to {}", min_fp, max_fp);
                Ok(Some((min_fp, max_fp)))
            }
        }
    }

    fn refresh_active_mode(&mut self) -> Result<()> {
        debug!("Refreshing active mode");
        let response = self.send_command(b"MMA", 1)?;
        self.mode = FromPrimitive::from_u8(response[0]);
        Ok(())
    }

    pub fn get_current(&mut self) -> Result<f64> {
        debug!("Getting current");
        let response = self.send_command(b"Ar\x00\x00", 2)?;
        let raw_current = i16::from_be_bytes([response[0], response[1]]) as f64;
        let current = raw_current * self.max_output_current / 4095.0;
        debug!("Current: {} mA", current);
        Ok(current)
    }

    pub fn set_current(&mut self, current: f64) -> Result<()> {
        debug!("Setting current to {} mA", current);
        if self.mode != Some(LensMode::Current) {
            return Err(LensError::WrongMode {
                expected: LensMode::Current,
                actual: self.mode,
            });
        }

        let raw_current = (current * 4095.0 / self.max_output_current) as i16;
        let mut cmd = Vec::from(&b"Aw"[..]);
        cmd.extend_from_slice(&raw_current.to_be_bytes());
        self.send_command(&cmd, 0).map(|_| ())
    }

    pub fn get_diopter(&mut self) -> Result<f64> {
        debug!("Getting diopter");
        let response = self.send_command(b"PrDA\x00\x00\x00\x00", 2)?;
        let raw_diopter = i16::from_be_bytes([response[0], response[1]]) as f64;
        let diopter = if self.firmware_type == "A" {
            raw_diopter / 200.0 - 5.0
        } else {
            raw_diopter / 200.0
        };
        debug!("Diopter: {}", diopter);
        Ok(diopter)
    }

    pub fn set_diopter(&mut self, diopter: f64) -> Result<()> {
        debug!("Setting diopter to {}", diopter);
        if self.mode != Some(LensMode::FocalPower) {
            return Err(LensError::WrongMode {
                expected: LensMode::FocalPower,
                actual: self.mode,
            });
        }

        let raw_diopter = if self.firmware_type == "A" {
            ((diopter + 5.0) * 200.0) as i16
        } else {
            (diopter * 200.0) as i16
        };

        let mut cmd = Vec::from(&b"PwDA"[..]);
        cmd.extend_from_slice(&raw_diopter.to_be_bytes());
        cmd.extend_from_slice(&[0, 0]);
        self.send_command(&cmd, 0).map(|_| ())
    }

    pub fn ramp_to_zero(&mut self, duration: f64, steps: usize) -> Result<()> {
        debug!("Ramping to zero over {} seconds with {} steps", duration, steps);
        
        let (start_value, set_func): (f64, fn(&mut Self, f64) -> Result<()>) = match self.mode {
            Some(LensMode::Current) => (self.get_current()?, Self::set_current),
            Some(LensMode::FocalPower) => (self.get_diopter()?, Self::set_diopter),
            None => {
                error!("Cannot ramp to zero: unknown mode");
                return Err(LensError::InvalidMode);
            }
        };

        self.ramp(start_value, 0.0, duration, steps, set_func)?;
        info!("Ramp to zero complete");
        Ok(())
    }

    fn ramp(&mut self, start: f64, end: f64, duration: f64, steps: usize,
            set_func: fn(&mut Self, f64) -> Result<()>) -> Result<()> {
        let step_size = (end - start) / steps as f64;
        let step_duration = Duration::from_secs_f64(duration / steps as f64);

        for i in 0..=steps {
            let target_value = start + (i as f64) * step_size;
            set_func(self, target_value)?;
            thread::sleep(step_duration);
        }
        Ok(())
    }

    fn send_command(&mut self, command: &[u8], reply_size: usize) -> Result<Vec<u8>> {
        let crc = self.calculate_crc_16(command);
        let mut cmd_with_crc = Vec::from(command);
        cmd_with_crc.extend_from_slice(&crc.to_le_bytes());

        debug!("Sending command: {:?}", cmd_with_crc);
        self.port.write_all(&cmd_with_crc)?;

        if reply_size == 0 {
            return Ok(Vec::new());
        }

        let mut response = vec![0u8; reply_size + 4];
        self.port.read_exact(&mut response)?;

        let (data, rest) = response.split_at(reply_size);
        let crc_received = u16::from_le_bytes([rest[0], rest[1]]);
        
        if crc_received != self.calculate_crc_16(data) || &rest[2..4] != b"\r\n" {
            return Err(LensError::CrcError);
        }

        Ok(data.to_vec())
    }

    fn calculate_crc_16(&self, data: &[u8]) -> u16 {
        let mut crc: u16 = 0;
        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if (crc & 1) > 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }
}

impl Drop for LensDriver {
    fn drop(&mut self) {
        info!("Dropping LensDriver");
        if let Err(e) = self.ramp_to_zero(1.0, 50) {
            error!("Error while ramping to zero during drop: {}", e);
        }
    }
}