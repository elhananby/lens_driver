use lens_driver::LensDriver;
use std::str;

fn main() {
    let mut opto = LensDriver::new(Some("/dev/ttyACM0".to_string()));
    opto.connect().expect("Failed to connect to the device");
    match opto.firmwaretype() {
        Ok(ft) => println!("Firmware type: {:?}", str::from_utf8(&[ft])),
        Err(e) => panic!("Failed to read firmware type: {}", e),
    }
}
