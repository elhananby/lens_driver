use pyo3::prelude::*;
use pyo3::exceptions;
use log::error;

use crate::driver::{LensDriver as RustLensDriver, LensMode, LensError};

#[pyclass]
pub struct PyLensDriver {
    inner: RustLensDriver,
}

#[pymethods]
impl PyLensDriver {
    #[new]
    fn new(port_name: &str, debug: bool) -> PyResult<Self> {
        RustLensDriver::new(port_name, debug)
            .map(|inner| PyLensDriver { inner })
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn get_mode(&self) -> PyResult<String> {
        match self.inner.mode() {
            Some(LensMode::Current) => Ok("current".to_string()),
            Some(LensMode::FocalPower) => Ok("focal_power".to_string()),
            None => Ok("unknown".to_string()),
        }
    }

    /// Get the current temperature of the lens
    fn get_temperature(&mut self) -> PyResult<f64> {
        self.inner
            .get_temperature()
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Set the operation mode of the lens
    /// 
    /// Args:
    ///     mode (str): Either "current" or "focal_power"
    /// 
    /// Returns:
    ///     Optional tuple of (min_fp, max_fp) when setting focal_power mode
    fn set_mode(&mut self, mode: &str) -> PyResult<Option<(f64, f64)>> {
        let lens_mode = match mode {
            "current" => LensMode::Current,
            "focal_power" => LensMode::FocalPower,
            _ => return Err(PyErr::new::<exceptions::PyValueError, _>(
                "Invalid mode. Choose 'current' or 'focal_power'."
            )),
        };

        self.inner
            .set_mode(lens_mode)
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }


    /// Get the current in mA
    fn get_current(&mut self) -> PyResult<f64> {
        self.inner
            .get_current()
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Set the current in mA
    fn set_current(&mut self, current: f64) -> PyResult<()> {
        self.inner
            .set_current(current)
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get the focal power in diopters
    fn get_diopter(&mut self) -> PyResult<f64> {
        self.inner
            .get_diopter()
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Set the focal power in diopters
    fn set_diopter(&mut self, diopter: f64) -> PyResult<()> {
        self.inner
            .set_diopter(diopter)
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Ramp the lens setting to zero over a specified duration
    /// 
    /// Args:
    ///     duration (float): Time in seconds over which to ramp
    ///     steps (int): Number of steps to use in the ramp
    fn ramp_to_zero(&mut self, duration: f64, steps: usize) -> PyResult<()> {
        self.inner
            .ramp_to_zero(duration, steps)
            .map_err(|e| PyErr::new::<exceptions::PyRuntimeError, _>(e.to_string()))
    }


    #[getter]
    fn firmware_type(&self) -> String {
        self.inner.firmware_type().to_string()
    }

    #[getter]
    fn firmware_version(&self) -> (u8, u8, u16, u16) {
        self.inner.firmware_version()
    }

    #[getter]
    fn max_output_current(&self) -> f64 {
        self.inner.max_output_current()
    }

    fn __repr__(&self) -> PyResult<String> {
        let mode = self.get_mode()?;
        Ok(format!("PyLensDriver(mode={})", mode))
    }

    fn __str__(&self) -> PyResult<String> {
        self.__repr__()
    }

    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[pyo3(signature = (exc_type=None, exc_value=None, traceback=None))]
    fn __exit__(
        &mut self,
        exc_type: Option<PyObject>,
        exc_value: Option<PyObject>,
        traceback: Option<PyObject>,
    ) {
        if let Err(e) = self.inner.ramp_to_zero(1.0, 50) {
            error!("Error during context manager exit: {}", e);
        }
    }
}


// Helper function to convert Rust errors to Python exceptions
fn to_py_err<E: std::error::Error>(err: E) -> PyErr {
    PyErr::new::<exceptions::PyRuntimeError, _>(err.to_string())
}

// Function to create better Python error messages from Rust errors
impl From<LensError> for PyErr {
    fn from(err: LensError) -> PyErr {
        match err {
            LensError::InvalidMode => {
                PyErr::new::<exceptions::PyValueError, _>("Invalid lens operation mode")
            }
            LensError::WrongMode { expected, actual } => {
                PyErr::new::<exceptions::PyValueError, _>(
                    format!("Wrong mode: expected {:?}, got {:?}", expected, actual)
                )
            }
            LensError::HandshakeFailed => {
                PyErr::new::<exceptions::PyConnectionError, _>("Device handshake failed")
            }
            LensError::CrcError => {
                PyErr::new::<exceptions::PyValueError, _>("CRC check failed on device response")
            }
            LensError::SerialPort(e) => {
                PyErr::new::<exceptions::PyIOError, _>(format!("Serial port error: {}", e))
            }
            LensError::Io(e) => {
                PyErr::new::<exceptions::PyIOError, _>(format!("IO error: {}", e))
            }
        }
    }
}