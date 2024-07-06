# Lens Driver

This project provides a Rust implementation for interfacing with a lens driver over a serial connection. It includes a `LensDriver` struct that handles communication, command sending, and response parsing, including CRC checks to ensure data integrity.
The implementation is pretty much based directly on the Python version from this repository:
[Opto](https://github.com/OrganicIrradiation/opto)

## Requirements

- Rust (latest stable version)
- A serial device to communicate with the lens driver
- A serial port library: [serialport](https://docs.rs/serialport/latest/serialport/)

