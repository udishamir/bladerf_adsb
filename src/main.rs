/*
    MIT License

    Copyright (c) 2025 Udi Shamir

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
*/

use adsb_deku::adsb::ME;
use adsb_deku::CPRFormat;
use adsb_deku::{Frame, DF};
use bladerf_adsb::{decode_bits, extract_messages, find_adsb_preambles, iq_magnitude};
use bladerf_sys::*;
use deku::DekuContainerRead;
use std::collections::HashMap;
use std::ptr;
use std::time::Instant;

#[derive(Clone)]
struct CprFrame {
    lat: u32,
    lon: u32,
    t: Instant,
}

#[derive(Default)]
struct CprState {
    even: Option<CprFrame>,
    odd: Option<CprFrame>,
}

fn nl(lat: f64) -> u32 {
    let lat_rad = lat.to_radians();
    let a = 1.0 - (1.0 - (std::f64::consts::PI / 30.0).cos()) / lat_rad.cos().powi(2);
    let result = (2.0 * std::f64::consts::PI / a.acos()).floor();

    if lat.abs() >= 87.0 {
        1 // Special case near the poles
    } else {
        result as u32
    }
}

fn decode_cpr_global(
    lat_even: u32,
    lon_even: u32,
    lat_odd: u32,
    lon_odd: u32,
    use_odd: bool,
) -> Option<(f64, f64)> {
    let yz_even = lat_even as f64 / 131072.0;
    let yz_odd = lat_odd as f64 / 131072.0;
    let j = ((59.0 * yz_even - 60.0 * yz_odd) + 0.5).floor();

    let rlat_even = 360.0 / 60.0 * ((j % 60.0) + yz_even);
    let rlat_odd = 360.0 / 59.0 * ((j % 59.0) + yz_odd);

    let rlat = if use_odd { rlat_odd } else { rlat_even };

    let nl_val = nl(rlat).max(1); // avoid divide-by-zero
    let ni = if use_odd {
        nl_val as f64
    } else {
        (nl_val - 1) as f64
    };

    let xz_even = lon_even as f64 / 131072.0;
    let xz_odd = lon_odd as f64 / 131072.0;
    let m = ((xz_even * nl_val as f64 - xz_odd * (nl_val as f64 + 1.0)) + 0.5).floor();

    let rlon = if use_odd {
        360.0 / ni * ((m % ni + xz_odd) % ni)
    } else {
        360.0 / ni * ((m % ni + xz_even) % ni)
    };

    Some((rlat, rlon))
}

fn binary_to_bytes(bits: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..bits.len())
        .step_by(8)
        .map(|i| u8::from_str_radix(&bits[i..i + 8], 2))
        .collect()
}

fn icao_bytes_to_hex(bytes: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", bytes[0], bytes[1], bytes[2])
}

fn process_adsb_frames(iq: &[i16]) -> Vec<(String, Vec<u16>)> {
    let magnitude = iq_magnitude(iq);
    let preambles = find_adsb_preambles(&magnitude);
    let message_windows = extract_messages(&magnitude, &preambles);
    let mut decoded_messages = Vec::new();
    for window in message_windows {
        if let Some(bits) = decode_bits(&window) {
            decoded_messages.push((bits, window));
        }
    }
    decoded_messages
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // I want to track Aircraft positioning and save it, each ICAO needs to have CPR pairs
    let mut cpr_map: HashMap<String, CprState> = HashMap::new();

    let mut dev: *mut bladerf = ptr::null_mut();
    if unsafe { bladerf_open(&mut dev, ptr::null()) } != 0 {
        return Err("Failed to open bladeRF device".into());
    }

    let rx_channel = bladerf_channel_layout_BLADERF_RX_X1;

    let ret = unsafe { bladerf_set_frequency(dev, rx_channel as i32, 1_090_000_000) };
    if ret != 0 {
        return Err(format!("Failed to set frequency: error {}", ret).into());
    }

    let ret = unsafe { bladerf_set_sample_rate(dev, rx_channel as i32, 2_000_000, &mut 0) };
    if ret != 0 {
        return Err(format!("Failed to set sample rate: error {}", ret).into());
    }

    let ret = unsafe { bladerf_set_bandwidth(dev, rx_channel as i32, 2_000_000, &mut 0) };
    if ret != 0 {
        return Err(format!("Failed to set bandwidth: error {}", ret).into());
    }

    let ret = unsafe { bladerf_set_gain(dev, rx_channel as i32, 70) };
    if ret != 0 {
        return Err(format!("Failed to set gain: error {}", ret).into());
    }

    let ret = unsafe {
        bladerf_sync_config(
            dev,
            rx_channel,
            bladerf_format_BLADERF_FORMAT_SC16_Q11,
            16,
            4096,
            8,
            3500,
        )
    };

    if ret != 0 {
        return Err(format!("Sync config failed: error {}", ret).into());
    }

    let ret = unsafe { bladerf_enable_module(dev, rx_channel as i32, true) };
    if ret != 0 {
        return Err(format!("Failed to enable RX module: error {}", ret).into());
    }

    println!("bladeRF initialized. Listening for ADS-B frames...");
    let mut buffer = vec![0i16; 20000];

    for _ in 0..10000 {
        let ret = unsafe {
            bladerf_sync_rx(
                dev,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                (buffer.len() / 2) as u32,
                ptr::null_mut(),
                5000,
            )
        };
        if ret < 0 {
            eprintln!("Failed to receive samples: {}", ret);
            break;
        }

        let decoded = process_adsb_frames(&buffer);
        for (bits, _) in decoded {
            if let Ok(bytes) = binary_to_bytes(&bits) {
                if let Ok((_, frame)) = Frame::from_bytes((&bytes[..], 0)) {
                    if let DF::ADSB(adsb_msg) = frame.df {
                        let icao = icao_bytes_to_hex(adsb_msg.icao.0);
                        let i_now = Instant::now();

                        match &adsb_msg.me {
                            ME::SurfacePosition(pos) => {
                                let now = i_now;
                                let lat = pos.lat_cpr;
                                let lon = pos.lon_cpr;

                                let new_cpr = CprFrame { lat, lon, t: now };
                                let entry = cpr_map.entry(icao.clone()).or_default();

                                match pos.f {
                                    CPRFormat::Even => entry.even = Some(new_cpr.clone()),
                                    CPRFormat::Odd => entry.odd = Some(new_cpr.clone()),
                                }

                                println!(
                                    "✈️ ICAO: {icao} | Stored {:?} CPR | lat_cpr={}, lon_cpr={}",
                                    pos.f, lat, lon
                                );

                                if let (Some(even), Some(odd)) = (&entry.even, &entry.odd) {
                                    if now.duration_since(even.t).as_secs() < 10
                                        && now.duration_since(odd.t).as_secs() < 10
                                    {
                                        let use_odd = odd.t > even.t;
                                        if let Some((lat, lon)) = decode_cpr_global(
                                            even.lat, even.lon, odd.lat, odd.lon, use_odd,
                                        ) {
                                            println!(
                                                "📍 ICAO: {icao} | LAT: {:.6}, LON: {:.6}",
                                                lat, lon
                                            );
                                        } else {
                                            println!("⚠️ ICAO: {icao} | CPR decode failed");
                                        }
                                    }
                                }
                            }

                            ME::AirbornePositionBaroAltitude(alt)
                            | ME::AirbornePositionGNSSAltitude(alt) => {
                                let now = i_now;
                                let lat = alt.lat_cpr;
                                let lon = alt.lon_cpr;

                                let new_cpr = CprFrame { lat, lon, t: now };
                                let entry = cpr_map.entry(icao.clone()).or_default();

                                match alt.odd_flag {
                                    CPRFormat::Even => entry.even = Some(new_cpr.clone()),
                                    CPRFormat::Odd => entry.odd = Some(new_cpr.clone()),
                                }

                                println!(
                                    "✈️ ICAO: {icao} | Stored {:?} CPR | lat_cpr={}, lon_cpr={}",
                                    alt.odd_flag, lat, lon
                                );

                                if let (Some(even), Some(odd)) = (&entry.even, &entry.odd) {
                                    if now.duration_since(even.t).as_secs() < 10
                                        && now.duration_since(odd.t).as_secs() < 10
                                    {
                                        println!(
                                            "✅ ICAO {icao} has both Even and Odd CPRs — attempting decode..."
                                        );

                                        let use_odd = odd.t > even.t;
                                        if let Some((lat, lon)) = decode_cpr_global(
                                            even.lat, even.lon, odd.lat, odd.lon, use_odd,
                                        ) {
                                            println!(
                                                "📍 ICAO: {icao} | LAT: {:.6}, LON: {:.6}",
                                                lat, lon
                                            );
                                        } else {
                                            println!("⚠️ ICAO: {icao} | CPR decode failed");
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        /*
        let count = 0;
        if count % 50 == 0 {
            println!("--- 🧠 CPR Map Debug ---");
            for (icao, state) in &cpr_map {
                println!(
                    "🧠 ICAO: {icao} | even: {} | odd: {}",
                    state.even.is_some(),
                    state.odd.is_some()
                );
            }
        }
        */
    }

    let ret = unsafe { bladerf_enable_module(dev, rx_channel as i32, false) };
    if ret != 0 {
        return Err(format!("Failed to disable RX module: error {}", ret).into());
    }

    unsafe { bladerf_close(dev) };
    Ok(())
}
