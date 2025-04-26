# Lightweight ADS-B (Mode S) Decoder for BladeRF SDR Devices, Drone Traffic Awareness Made Simple

## Author

Developed with passion by **Udi Shamir**.  
Feel free to reach out or contribute!

- GitHub: [udishamir](https://github.com/udishamir)

### About

BladeRF_adsb is a Rust project designed to simplify the processing of ADS-B signals captured from BladeRF SDR hardware.
It enables the easy capture of air traffic data, including aircraft positions, altitudes, and IDs with minimal setup and maximum performance.

At this stage, the project leverages (Nuand’s bladeRF-adsb)[https://www.nuand.com/product/bladeRF-xA9/] tools for the low-level IQ demodulation of 1090 MHz ADS-B transmissions.
Rust code then takes over to process, extract, and handle the aircraft data natively.

My primary incentive is to make ADS-B reception accessible, efficient, and drone-oriented, by offering a clean, modern, and minimalistic software stack.

Note:This project currently depends on Nuand’s (bladeRF-adsb)[https://github.com/Nuand/bladeRF-adsb] for SDR demodulation, but handles all data processing natively in Rust.

## Modus Operandi

* BladeRF SDR hardware captures raw IQ samples centered on 1090 MHz (the ADS-B band).
* The external tool bladeRF-adsb (C-based) demodulates these IQ samples into discrete Mode S / ADS-B frames.
* bladerf_adsb (this Rust project) then parses, validates, and structures these ADS-B frames into native Rust data types (aircraft ICAO, position, velocity, etc.).
* Future plans include direct processing of IQ samples, removing the dependency on bladeRF-adsb.

## Code Structure

* **src/lib.rs**: Defines the core [`AdsbMessage`](https://mode-s.org/1090mhz/content/ads-b/1-basics.html) data structure representing aircraft ADS-B information.
  Provides a modular Rust-native API to work with decoded flight data.

* **src/main.rs**: Implements the live runtime.
  Connects to an external BladeRF ADS-B demodulator, parses ADS-B messages, and structures them for further processing.

+-----------------+            +-------------------------+           +----------------------+
|  BladeRF SDR    |  → RF IQ →  | bladeRF-adsb (C Demod)   |  → ADS-B Frames → | bladerf_adsb (Rust) |
| (Hardware 1090 MHz) |         | (Demodulate / Sync frames) |          | (Parse and structure aircraft data) |
+-----------------+            +-------------------------+           +----------------------+



![Screenshot 2025-04-26 at 8 21 44 AM](https://github.com/user-attachments/assets/a20393c5-8aad-4754-a8a1-8fafecf2561c)



## Future Roadmap
| Feature | Status | Notes |
|:--------|:------:|:------|
| Full Compact Position Reporting (CPR) Decoding | - | Enable precise Latitude/Longitude from airborne ADS-B frames |
| Aircraft Velocity and Heading Extraction | V | Alr- supported (basic parsing done) |
| Squawk Code and Flight ID Extraction | - | Parse emergency codes and flight identifiers |
| Direct IQ Demodulation in Rust | - | Eliminate dependency on external `bladeRF-adsb` |
| Multi-aircraft Tracking and Timeout Cleanup | - | Maintain a live list of detected aircraft |
| JSON Export | - | Stream parsed aircraft data in structured JSON format |
| WebSocket Broadcast | - | Enable real-time aircraft tracking over network |
| Drone System Integrity | - | Feed ADS-B traffic data into drone flight control for collision avoidance |
| Minimal Resource Mode fo- SBCs      | - | Optimize for Raspberry Pi / Jetson class devices |
