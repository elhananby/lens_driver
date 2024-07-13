# Rust Lens Control Library

This library provides a Rust implementation for controlling optical lenses via serial communication. It's based on a Python implementation and offers a robust, type-safe interface for lens operations.

## Features

- Serial communication with lens hardware
- Firmware type and version retrieval
- Temperature control and monitoring
- Current and diopter adjustments
- EEPROM read/write operations
- Comprehensive error handling

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
lens_driver = "0.1.0"
```

Then, add this to your crate root:

```rust
use lens_driver::Lens;
```

## Usage

Here's a basic example of how to use the Lens Control Library:

```rust
use lens_driver::Lens;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a new Lens object
    let mut lens = Lens::new("/dev/ttyUSB0", false)?;

    // Get the current temperature
    let temp = lens.get_temperature()?;
    println!("Current temperature: {:.2}°C", temp);

    // Set temperature limits
    lens.set_temperature_limits(20.0, 40.0)?;

    // Switch to focal power mode
    let (min_fp, max_fp) = lens.to_focal_power_mode()?;
    println!("Focal power range: {:.2} to {:.2} diopters", min_fp, max_fp);

    // Set diopter
    lens.set_diopter(2.5)?;

    // Get current diopter
    let diopter = lens.get_diopter()?;
    println!("Current diopter: {:.2}", diopter);

    Ok(())
}
```

## API Reference

### `Lens::new(port_name: &str, debug: bool) -> Result<Lens, LensError>`

Creates a new `Lens` object and initializes communication with the lens hardware.

### `get_temperature() -> Result<f32, LensError>`

Retrieves the current temperature of the lens.

### `set_temperature_limits(lower: f32, upper: f32) -> Result<(u8, f32, f32), LensError>`

Sets the lower and upper temperature limits for the lens.

### `get_current() -> Result<f32, LensError>`

Retrieves the current applied to the lens.

### `set_current(current: f32) -> Result<(), LensError>`

Sets the current to be applied to the lens.

### `get_diopter() -> Result<f32, LensError>`

Retrieves the current diopter setting of the lens.

### `set_diopter(diopter: f32) -> Result<(), LensError>`

Sets the diopter of the lens.

### `to_focal_power_mode() -> Result<(f32, f32), LensError>`

Switches the lens to focal power mode and returns the minimum and maximum focal power range.

### `to_current_mode() -> Result<(), LensError>`

Switches the lens to current mode.

For a complete list of methods and their descriptions, please refer to the API documentation.

## Error Handling

The library uses a custom `LensError` enum for error handling. This enum covers various error scenarios such as serial port errors, CRC mismatches, and invalid modes.

## Testing

The library comes with a comprehensive test suite. To run the tests, use:

```
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- This project is based on a Python implementation of lens control software, as provided by Optotune.