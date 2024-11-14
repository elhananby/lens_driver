use lens_driver::{LensDriver, LensMode, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut driver = LensDriver::new("/dev/optotune_ld", true)?;
    
    println!("Getting temperature...");
    let temp = driver.get_temperature()?;
    println!("Temperature: {}°C", temp);
    
    println!("Setting current mode...");
    driver.set_mode(LensMode::Current)?;
    driver.set_current(50.0)?;
    
    println!("Setting focal power mode...");
    if let Some((min_fp, max_fp)) = driver.set_mode(LensMode::FocalPower)? {
        println!("Focal power range: {} to {}", min_fp, max_fp);
    }
    
    Ok(())
}