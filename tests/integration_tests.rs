#[cfg(test)]
mod tests {
    use lens_driver::{LensDriver, LensMode};

    #[test]
    fn test_temperature_reading() {
        let mut driver = LensDriver::new("/dev/optotune_ld", false).unwrap();
        let temp = driver.get_temperature().unwrap();
        assert!(temp > -40.0 && temp < 125.0);
    }

    // Add more tests...
}
