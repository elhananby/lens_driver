use pyo3::prelude::*;

mod driver;
mod python;

pub use driver::*;

/// Entry point for the Python module
#[pymodule]
fn lens_driver(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<python::PyLensDriver>()?;
    Ok(())
}