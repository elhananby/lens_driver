from lens_driver import PyLensDriver

def main():
    with PyLensDriver("/dev/optotune_ld", debug=True) as lens:
        print("Getting temperature...")
        temp = lens.get_temperature()
        print(f"Temperature: {temp}°C")
        
        print("Setting current mode...")
        lens.set_mode("current")
        lens.set_current(50.0)
        
        print("Setting focal power mode...")
        result = lens.set_mode("focal_power")
        if result:
            min_fp, max_fp = result
            print(f"Focal power range: {min_fp} to {max_fp}")

if __name__ == "__main__":
    main()