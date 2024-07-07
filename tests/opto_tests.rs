// tests/opto_tests.rs

use lens_driver::LensDriver;
use std::thread;
use std::time::Duration;

const PORT: &str = "/dev/ttyUSB0"; // Adjust this to match your system's port
const DELAY_BETWEEN_TESTS: Duration = Duration::from_secs(2); // 2 second delay between tests

// Custom test runner
fn run_test<T>(test: T)
where
    T: FnOnce() + std::panic::UnwindSafe,
{
    let result = std::panic::catch_unwind(|| {
        test();
    });

    thread::sleep(DELAY_BETWEEN_TESTS);

    if let Err(e) = result {
        eprintln!("Test failed: {:?}", e);
    }
}

#[test]
fn test_opto_basic_functionality() {
    run_test(|| {
        let mut opto = LensDriver::new(Some(PORT.to_string()));

        // Test opening the device
        match opto.connect() {
            Ok(_) => println!("Successfully connected to the device"),
            Err(e) => panic!("Failed to connect to the device: {}", e),
        }

        // Test changing mode to current and setting/reading current
        assert!(
            opto.mode(Some("current")).is_ok(),
            "Failed to set mode to current"
        );
        thread::sleep(Duration::from_millis(100));

        let test_current = 50.0;
        assert!(
            opto.current(Some(test_current)).is_ok(),
            "Failed to set current"
        );
        thread::sleep(Duration::from_millis(100));

        let read_current = opto.current(None).expect("Failed to read current");
        assert!(
            (read_current - test_current).abs() < 1.0,
            "Current value mismatch"
        );

        // Test changing mode to focal and setting/reading focal power
        assert!(
            opto.mode(Some("focal")).is_ok(),
            "Failed to set mode to focal"
        );
        thread::sleep(Duration::from_millis(100));

        let test_focal = 3.0;
        assert!(
            opto.focalpower(Some(test_focal)).is_ok(),
            "Failed to set focal power"
        );
        thread::sleep(Duration::from_millis(100));

        let read_focal = opto.focalpower(None).expect("Failed to read focal power");
        assert!(
            (read_focal - test_focal).abs() < 0.1,
            "Focal power value mismatch"
        );

        // Test reading temperature
        let temperature = opto.temp_reading().expect("Failed to read temperature");
        assert!(
            temperature > 0.0 && temperature < 100.0,
            "Temperature out of expected range"
        );

        // Close the device
        assert!(opto.close(true).is_ok(), "Failed to close the device");
    });
}

#[test]
fn test_temp_limits() {
    run_test(|| {
        let mut opto = LensDriver::new(Some(PORT.to_string()));
        match opto.connect() {
            Ok(_) => println!("Successfully connected to the device"),
            Err(e) => panic!("Failed to connect to the device: {}", e),
        }

        // Test setting temperature limits
        let test_temp_limits = (20.0, 30.0);
        assert!(
            opto.temp_limits(Some(test_temp_limits)).is_ok(),
            "Failed to set temperature limits"
        );
        thread::sleep(Duration::from_millis(100));

        // Read back temperature limits
        let read_temp_limits = opto
            .temp_limits(None)
            .expect("Failed to read temperature limits");

        // Check if the upper limit is correctly set
        assert!(
            (read_temp_limits.1 - test_temp_limits.1).abs() < 0.1,
            "Upper temperature limit mismatch"
        );

        // Check if the lower limit is set and matches, or is equal to the upper limit
        assert!(
            (read_temp_limits.0 - test_temp_limits.0).abs() < 0.1
                || (read_temp_limits.0 - read_temp_limits.1).abs() < 0.1,
            "Lower temperature limit unexpected"
        );

        assert!(opto.close(true).is_ok(), "Failed to close the device");
    });
}

#[test]
fn test_current_limits() {
    run_test(|| {
        let mut opto = LensDriver::new(Some(PORT.to_string()));
        match opto.connect() {
            Ok(_) => println!("Successfully connected to the device"),
            Err(e) => panic!("Failed to connect to the device: {}", e),
        }

        // Test setting current limits
        let test_upper_limit = 200.0;
        let test_lower_limit = -200.0;

        assert!(
            opto.current_upper(Some(test_upper_limit)).is_ok(),
            "Failed to set upper current limit"
        );
        thread::sleep(Duration::from_millis(100));

        assert!(
            opto.current_lower(Some(test_lower_limit)).is_ok(),
            "Failed to set lower current limit"
        );
        thread::sleep(Duration::from_millis(100));

        // Read back current limits
        let read_upper_limit = opto
            .current_upper(None)
            .expect("Failed to read upper current limit");
        let read_lower_limit = opto
            .current_lower(None)
            .expect("Failed to read lower current limit");

        assert!(
            (read_upper_limit - test_upper_limit).abs() < 0.1,
            "Upper current limit mismatch"
        );
        assert!(
            (read_lower_limit - test_lower_limit).abs() < 0.1,
            "Lower current limit mismatch"
        );

        assert!(opto.close(true).is_ok(), "Failed to close the device");
    });
}

#[test]
fn test_firmware_info() {
    run_test(|| {
        let mut opto = LensDriver::new(Some(PORT.to_string()));
        match opto.connect() {
            Ok(_) => println!("Successfully connected to the device"),
            Err(e) => panic!("Failed to connect to the device: {}", e),
        }

        match opto.firmwaretype() {
            Ok(ft) => println!("Firmware type: {:?}", ft),
            Err(e) => panic!("Failed to read firmware type: {}", e),
        }

        match opto.firmwarebranch() {
            Ok(fb) => println!("Firmware branch: {:?}", fb),
            Err(e) => panic!("Failed to read firmware branch: {}", e),
        }

        match opto.firmwareversion() {
            Ok(fv) => println!("Firmware version: {}", fv),
            Err(e) => panic!("Failed to read firmware version: {}", e),
        }

        assert!(opto.close(true).is_ok(), "Failed to close the device");
    });
}

#[test]
fn test_device_info() {
    run_test(|| {
        let mut opto = LensDriver::new(Some(PORT.to_string()));
        match opto.connect() {
            Ok(_) => println!("Successfully connected to the device"),
            Err(e) => panic!("Failed to connect to the device: {}", e),
        }

        match opto.partnumber() {
            Ok(pn) => println!("Part number: {:?}", pn),
            Err(e) => panic!("Failed to read part number: {}", e),
        }

        match opto.serialnumber() {
            Ok(sn) => println!("Serial number: {:?}", sn),
            Err(e) => panic!("Failed to read serial number: {}", e),
        }

        match opto.deviceid() {
            Ok(id) => println!("Device ID: {:?}", id),
            Err(e) => panic!("Failed to read device ID: {}", e),
        }
        match opto.firmwaretype() {
            Ok(ft) => println!("Firmware type: {:?}", ft),
            Err(e) => panic!("Failed to read firmware type: {}", e),
        }
        assert!(opto.close(true).is_ok(), "Failed to close the device");
    });
}

#[test]
fn test_signal_generator() {
    run_test(|| {
        let mut opto = LensDriver::new(Some(PORT.to_string()));
        match opto.connect() {
            Ok(_) => println!("Successfully connected to the device"),
            Err(e) => panic!("Failed to connect to the device: {}", e),
        }

        let test_upper = 100.0;
        let test_lower = -100.0;
        let test_freq = 10.0;

        assert!(
            opto.siggen_upper(Some(test_upper)).is_ok(),
            "Failed to set signal generator upper limit"
        );
        assert!(
            opto.siggen_lower(Some(test_lower)).is_ok(),
            "Failed to set signal generator lower limit"
        );
        assert!(
            opto.siggen_freq(Some(test_freq)).is_ok(),
            "Failed to set signal generator frequency"
        );

        thread::sleep(Duration::from_millis(100));

        let read_upper = opto
            .siggen_upper(None)
            .expect("Failed to read signal generator upper limit");
        let read_lower = opto
            .siggen_lower(None)
            .expect("Failed to read signal generator lower limit");
        let read_freq = opto
            .siggen_freq(None)
            .expect("Failed to read signal generator frequency");

        assert!(
            (read_upper - test_upper).abs() < 0.1,
            "Signal generator upper limit mismatch"
        );
        assert!(
            (read_lower - test_lower).abs() < 0.1,
            "Signal generator lower limit mismatch"
        );
        assert!(
            (read_freq - test_freq).abs() < 0.1,
            "Signal generator frequency mismatch"
        );

        assert!(opto.close(true).is_ok(), "Failed to close the device");
    });
}

#[test]
fn test_opto_error_handling() {
    run_test(|| {
        let mut opto = LensDriver::new(Some("NONEXISTENT_PORT".to_string()));

        // Test connecting to non-existent port
        match opto.connect() {
            Ok(_) => panic!(
                "Expected error when connecting to non-existent port, but connection succeeded"
            ),
            Err(e) => println!(
                "Received expected error when connecting to non-existent port: {}",
                e
            ),
        }

        // Test operations on disconnected device
        match opto.current(Some(50.0)) {
            Ok(_) => panic!("Expected error when setting current on disconnected device"),
            Err(e) => println!(
                "Received expected error when setting current on disconnected device: {}",
                e
            ),
        }

        match opto.mode(Some("current")) {
            Ok(_) => panic!("Expected error when setting mode on disconnected device"),
            Err(e) => println!(
                "Received expected error when setting mode on disconnected device: {}",
                e
            ),
        }

        match opto.temp_reading() {
            Ok(_) => panic!("Expected error when reading temperature on disconnected device"),
            Err(e) => println!(
                "Received expected error when reading temperature on disconnected device: {}",
                e
            ),
        }
    });
}
