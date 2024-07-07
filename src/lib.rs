// src/lib.rs

use serialport::SerialPort;
use std::error::Error;
use std::time::Duration;
use std::{fmt, str};

#[derive(Debug)]
pub enum LensError {
    SerialError(serialport::Error),
    HandshakeError,
    IoError(std::io::Error),
}

impl fmt::Display for LensError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LensError::SerialError(e) => write!(f, "Serial port error: {}", e),
            LensError::HandshakeError => write!(f, "Handshake failed"),
            LensError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl Error for LensError {}

impl From<serialport::Error> for LensError {
    fn from(error: serialport::Error) -> Self {
        LensError::SerialError(error)
    }
}

impl From<std::io::Error> for LensError {
    fn from(error: std::io::Error) -> Self {
        LensError::IoError(error)
    }
}

pub struct LensDriver {
    port: Option<String>,
    firmware_type: Option<u8>,
    crc_table: Vec<u16>,
    ser: Option<Box<dyn SerialPort>>,
    current: Option<f64>,
    current_max: f64,
}

impl LensDriver {
    pub fn new(port: Option<String>) -> Self {
        LensDriver {
            port,
            firmware_type: None,
            crc_table: Self::init_crc_table(),
            ser: None,
            current: None,
            current_max: 292.84,
        }
    }

    fn init_crc_table() -> Vec<u16> {
        let polynomial: u16 = 0xA001;
        let mut table = vec![0u16; 256];

        for i in 0..256 {
            let mut crc = i as u16;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ polynomial;
                } else {
                    crc >>= 1;
                }
            }
            table[i] = crc;
        }

        table
    }

    pub fn connect(&mut self) -> Result<(), LensError> {
        let port = self
            .port
            .as_ref()
            .ok_or(LensError::SerialError(serialport::Error::new(
                serialport::ErrorKind::NoDevice,
                "Port not set",
            )))?;
        let mut port = serialport::new(port, 115_200)
            .timeout(Duration::from_millis(200))
            .open()?;

        port.write_all(b"Start")?;
        let mut buffer = [0; 7];
        port.read_exact(&mut buffer)?;

        if &buffer != b"Ready\r\n" {
            return Err(LensError::HandshakeError);
        }

        self.ser = Some(port);
        self.firmware_type = self.firmwaretype().ok();
        Ok(())
    }

    pub fn close(&mut self, soft_close: bool) -> Result<(), Box<dyn Error>> {
        if let Some(_ser) = &mut self.ser {
            if let Some(current) = self.current {
                if soft_close {
                    for _ in 0..5 {
                        self.current(Some(current / 2.0))?;
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    self.current(Some(0.0))?;
                }
            }
            self.ser = None;
        }
        Ok(())
    }

    fn send_cmd(
        &mut self,
        cmd: &[u8],
        include_crc: bool,
        wait_for_resp: bool,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut full_cmd = cmd.to_vec();
        if include_crc {
            let crc = self.calc_crc(&full_cmd);
            full_cmd.extend_from_slice(&crc.to_le_bytes());
        }

        let ser = self.ser.as_mut().ok_or("Serial not connected")?;
        ser.write_all(&full_cmd)?;

        if wait_for_resp {
            let mut resp = Vec::new();
            let mut buffer = [0; 1];
            loop {
                match ser.read(&mut buffer) {
                    Ok(0) => break, // End of input
                    Ok(_) => {
                        resp.push(buffer[0]);
                        if buffer[0] == b'\n' {
                            break;
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                    Err(e) => return Err(Box::new(e)),
                }
            }

            if include_crc {
                if resp.len() < 4 {
                    return Err("Response too short for CRC check".into());
                }
                let resp_crc = u16::from_le_bytes([resp[resp.len() - 4], resp[resp.len() - 3]]);
                let resp_content = &resp[..resp.len() - 4];
                if resp_crc != self.calc_crc(resp_content) {
                    return Err("CRC mismatch".into());
                }
                resp = resp_content.to_vec();
            }

            if resp.first() == Some(&b'E') {
                return Err(format!("Command error: {:?}", resp).into());
            }

            Ok(resp)
        } else {
            Ok(Vec::new())
        }
    }

    fn calc_crc(&self, data: &[u8]) -> u16 {
        let mut crc: u16 = 0;
        for &d in data {
            let tmp = crc ^ (d as u16);
            crc = (crc >> 8) ^ self.crc_table[(tmp & 0x00ff) as usize];
        }
        crc
    }

    pub fn current(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                let data = ((val * 4095.0 / self.current_max) as i16).to_be_bytes();
                self.send_cmd(&[b'A', b'w', data[0], data[1]], true, false)?;
                self.current = Some(val);
            }
            None => {
                let resp = self.send_cmd(b"Ar\x00\x00", true, true)?;
                self.current =
                    Some(i16::from_be_bytes([resp[1], resp[2]]) as f64 * self.current_max / 4095.0);
            }
        }
        Ok(self.current.unwrap())
    }

    pub fn handshake(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.send_cmd(b"Start", false, true)
    }

    pub fn firmwaretype(&mut self) -> Result<u8, Box<dyn Error>> {
        let resp = self.send_cmd(b"H", true, true)?;
        Ok(resp[1])
    }

    pub fn firmwarebranch(&mut self) -> Result<u8, Box<dyn Error>> {
        let resp = self.send_cmd(b"F", true, true)?;
        Ok(resp[1])
    }

    pub fn partnumber(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let resp = self.send_cmd(b"J", true, true)?;
        Ok(resp[1..4].to_vec())
    }

    pub fn current_upper(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                if val > self.current_max {
                    return Err("Limit cannot be higher than the maximum output current.".into());
                }
                let data = ((val * 4095.0 / self.current_max) as u16).to_be_bytes();
                self.send_cmd(&[b'C', b'w', b'U', b'A', data[0], data[1]], true, true)?;
            }
            None => {
                self.send_cmd(b"CrUA\x00\x00", true, true)?;
            }
        }
        let resp = self.send_cmd(b"CrUA\x00\x00", true, true)?;
        Ok(u16::from_be_bytes([resp[3], resp[4]]) as f64 * self.current_max / 4095.0)
    }

    pub fn current_lower(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                if val > self.current_max {
                    return Err("Limit cannot be higher than the maximum output current.".into());
                }
                let data = ((val * 4095.0 / self.current_max) as u16).to_be_bytes();
                self.send_cmd(&[b'C', b'w', b'L', b'A', data[0], data[1]], true, true)?;
            }
            None => {
                self.send_cmd(b"CrLA\x00\x00", true, true)?;
            }
        }
        let resp = self.send_cmd(b"CrLA\x00\x00", true, true)?;
        Ok(u16::from_be_bytes([resp[3], resp[4]]) as f64 * self.current_max / 4095.0)
    }

    pub fn firmwareversion(&mut self) -> Result<String, Box<dyn Error>> {
        let resp = self.send_cmd(b"V", true, true)?;
        Ok(format!(
            "{}.{}.{}.{}",
            resp[1],
            resp[2],
            u16::from_be_bytes([resp[3], resp[4]]),
            u16::from_be_bytes([resp[5], resp[6]]),
        ))
    }

    pub fn deviceid(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let resp = self.send_cmd(b"IR\x00\x00\x00\x00\x00\x00\x00\x00", true, true)?;
        Ok(resp[2..].to_vec())
    }

    pub fn gain(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                if !(0.0..=5.0).contains(&val) {
                    return Err("Gain must be between 0 and 5.".into());
                }
                let data = (val * 100.0) as u16;
                let resp = self.send_cmd(
                    &[b'O', b'w', data.to_be_bytes()[0], data.to_be_bytes()[1]],
                    true,
                    true,
                )?;
                let status = resp[2];
                let focal_max = u16::from_be_bytes([resp[3], resp[4]]) as f64 / 200.0;
                let focal_min = u16::from_be_bytes([resp[5], resp[6]]) as f64 / 200.0;
                println!(
                    "Status: {}, Focal range: {} to {}",
                    status, focal_min, focal_max
                );
                Ok(val)
            }
            None => {
                let resp = self.send_cmd(b"Or\x00\x00", true, true)?;
                Ok(u16::from_be_bytes([resp[2], resp[3]]) as f64 / 100.0)
            }
        }
    }

    pub fn serialnumber(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let resp = self.send_cmd(b"X", true, true)?;
        Ok(resp[1..].to_vec())
    }

    pub fn siggen_upper(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                let data = ((val * 4095.0 / self.current_max) as i16).to_be_bytes();
                self.send_cmd(
                    &[b'P', b'w', b'U', b'A', data[0], data[1], 0, 0],
                    true,
                    false,
                )?;
                Ok(val)
            }
            None => {
                let resp = self.send_cmd(b"PrUA\x00\x00\x00\x00", true, true)?;
                Ok(i16::from_be_bytes([resp[3], resp[4]]) as f64 * self.current_max / 4095.0)
            }
        }
    }

    pub fn siggen_lower(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                let data = ((val * 4095.0 / self.current_max) as i16).to_be_bytes();
                self.send_cmd(
                    &[b'P', b'w', b'L', b'A', data[0], data[1], 0, 0],
                    true,
                    false,
                )?;
                Ok(val)
            }
            None => {
                let resp = self.send_cmd(b"PrLA\x00\x00\x00\x00", true, true)?;
                Ok(i16::from_be_bytes([resp[3], resp[4]]) as f64 * self.current_max / 4095.0)
            }
        }
    }

    pub fn siggen_freq(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                let data = (val * 1000.0) as u32;
                let bytes = data.to_be_bytes();
                self.send_cmd(
                    &[
                        b'P', b'w', b'F', b'A', bytes[0], bytes[1], bytes[2], bytes[3],
                    ],
                    true,
                    false,
                )?;
                Ok(val)
            }
            None => {
                let resp = self.send_cmd(b"PrFA\x00\x00\x00\x00", true, true)?;
                Ok(u32::from_be_bytes([resp[3], resp[4], resp[5], resp[6]]) as f64 / 1000.0)
            }
        }
    }

    pub fn temp_reading(&mut self) -> Result<f64, Box<dyn Error>> {
        let resp = self.send_cmd(b"TCA", true, true)?;
        Ok(i16::from_be_bytes([resp[3], resp[4]]) as f64 * 0.0625)
    }

    pub fn mode(&mut self, mode_str: Option<&str>) -> Result<String, Box<dyn Error>> {
        match mode_str {
            Some(mode) => {
                let cmd = match mode {
                    "sinusoidal" => b"MwSA",
                    "rectangular" => b"MwQA",
                    "current" => b"MwDA",
                    "triangular" => b"MwTA",
                    "focal" => b"MwCA",
                    "analog" => b"MwAA",
                    _ => return Err(format!("Invalid mode: {}", mode).into()),
                };
                self.send_cmd(cmd, true, true)?;
                Ok(mode.to_string())
            }
            None => {
                let resp = self.send_cmd(b"MMA", true, true)?;
                let mode = match resp[3] {
                    1 => "current",
                    2 => "sinusoidal",
                    3 => "triangular",
                    4 => "rectangular",
                    5 => "focal",
                    6 => "analog",
                    7 => "position",
                    _ => "unknown",
                };
                Ok(mode.to_string())
            }
        }
    }

    pub fn focalpower(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                let data = ((val * 200.0) as i16).to_be_bytes();
                self.send_cmd(
                    &[b'P', b'w', b'D', b'A', data[0], data[1], 0, 0],
                    true,
                    false,
                )?;
                Ok(val)
            }
            None => {
                let resp = self.send_cmd(b"PrDA\x00\x00\x00\x00", true, true)?;
                Ok(i16::from_be_bytes([resp[2], resp[3]]) as f64 / 200.0 - 5.0)
            }
        }
    }

    pub fn temp_limits(&mut self, value: Option<(f64, f64)>) -> Result<(f64, f64), Box<dyn Error>> {
        match value {
            Some((lower, upper)) => {
                if lower > upper {
                    return Err("Lower temperature limit must be less than upper limit".into());
                }
                let lower_data = ((lower * 16.0) as i16).to_be_bytes();
                let upper_data = ((upper * 16.0) as i16).to_be_bytes();
                self.send_cmd(
                    &[
                        b'P',
                        b'w',
                        b'T',
                        b'A',
                        upper_data[0],
                        upper_data[1],
                        lower_data[0],
                        lower_data[1],
                    ],
                    true,
                    true,
                )?;
            }
            None => {}
        }
        let resp = self.send_cmd(b"PrTA\x00\x00\x00\x00", true, true)?;

        if resp.len() < 5 {
            return Err("Unexpected response length when reading temperature limits".into());
        }

        let upper = i16::from_be_bytes([resp[3], resp[4]]) as f64 / 200.0 - 5.0;

        let lower = if resp.len() >= 7 {
            i16::from_be_bytes([resp[5], resp[6]]) as f64 / 200.0 - 5.0
        } else {
            // If lower limit is not provided, we'll assume it's the same as the upper limit
            // You might want to adjust this behavior based on your device's actual behavior
            upper
        };

        Ok((lower, upper))
    }

    pub fn current_max(&mut self, value: Option<f64>) -> Result<f64, Box<dyn Error>> {
        match value {
            Some(val) => {
                if val > 292.84 {
                    return Err("Maximum current cannot exceed 292.84 mA".into());
                }
                let data = (val * 100.0) as u16;
                self.send_cmd(
                    &[
                        b'C',
                        b'w',
                        b'M',
                        b'A',
                        data.to_be_bytes()[0],
                        data.to_be_bytes()[1],
                    ],
                    true,
                    true,
                )?;
                self.current_max = val;
            }
            None => {
                let resp = self.send_cmd(b"CrMA\x00\x00", true, true)?;
                self.current_max = u16::from_be_bytes([resp[3], resp[4]]) as f64 / 100.0;
            }
        }
        Ok(self.current_max)
    }
}

impl Drop for LensDriver {
    fn drop(&mut self) {
        let _ = self.close(true);
    }
}
