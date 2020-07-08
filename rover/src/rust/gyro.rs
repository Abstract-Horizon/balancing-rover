//
// Copyright (C) 2020 Abstract Horizon
// All rights reserved. This program and the accompanying materials
// are made available under the terms of the Apache License v2.0
// which accompanies this distribution, and is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
//  Contributors:
//    Daniel Sendula - initial API and implementation
//

use std::time::{SystemTime, UNIX_EPOCH};

use byteorder::{ByteOrder, LittleEndian};

use phf::phf_map;

use rppal::i2c::I2c;


const CTRL_REG1: u8 = 0x20;
const CTRL_REG2: u8 = 0x21;
const CTRL_REG3: u8 = 0x22;
const CTRL_REG4: u8 = 0x23;
const CTRL_REG5: u8 = 0x24;
//const REFERENCE: u8 = 0x25;
//const OUT_TEMP: u8 = 0x26;
const STATUS_REG: u8 = 0x27;
const OUT_X_L: u8 = 0x28;
//const OUT_X_H: u8 = 0x29;
//const OUT_Y_L: u8 = 0x2A;
//const OUT_Y_H: u8 = 0x2B;
//const OUT_Z_L: u8 = 0x2C;
//const OUT_Z_H: u8 = 0x2D;
const FIFO_CTRL_REG: u8 = 0x2E;
const FIFO_SRC_REG: u8 = 0x2F;
//const INT1_CFG: u8 = 0x30;
//const INT1_SRC: u8 = 0x31;
//const INT1_TSH_XH: u8 = 0x32;
//const INT1_TSH_XL: u8 = 0x33;
//const INT1_TSH_YH: u8 = 0x34;
//const INT1_TSH_YL: u8 = 0x35;
//const INT1_TSH_ZH: u8 = 0x36;
//const INT1_TSH_ZL: u8 = 0x37;
//const INT1_DURATION: u8 = 0x38;


/*
const FREQ_BANDWIDTH_100_12_5: u8 = 0x00;
const FREQ_BANDWIDTH_100_25: u8 = 0x10;
const FREQ_BANDWIDTH_200_12_5: u8 = 0x40;
const FREQ_BANDWIDTH_200_25: u8 = 0x50;
const FREQ_BANDWIDTH_200_50: u8 = 0x60;
const FREQ_BANDWIDTH_200_70: u8 = 0x70;
const FREQ_BANDWIDTH_400_20: u8 = 0x80;
const FREQ_BANDWIDTH_400_25: u8 = 0x90;
const FREQ_BANDWIDTH_400_50: u8 = 0xA0;
const FREQ_BANDWIDTH_400_110: u8 = 0xB0;
const FREQ_BANDWIDTH_800_30: u8 = 0xC0;
const FREQ_BANDWIDTH_800_35: u8 = 0xD0;
const FREQ_BANDWIDTH_800_50: u8 = 0xE0;
const FREQ_BANDWIDTH_800_111: u8 = 0xF0;
*/

// #[derive(Clone)]
pub struct DataPoint {
    dx: i16,
    dy: i16,
    dz: i16,
    status: u16,
    fifo_status: u8
}

impl DataPoint {
    fn empty() -> DataPoint {
        DataPoint { dx: 0, dy: 0, dz: 0, status: 0, fifo_status: 0 }
    }

    fn new(dx: i16, dy: i16, dz: i16, status: u16, fifo_status: u8) -> DataPoint {
        DataPoint { dx, dy, dz, status, fifo_status }
    }
}

const FREQ_100: phf::Map<&'static str, u8> = phf_map! {"_" => 0x00, "12.5" => 0, "25" => 0x10};
const FREQ_200: phf::Map<&'static str, u8> = phf_map! {"_" => 0x40, "12.5" => 0, "25" => 0x10, "50" => 0x20, "70" => 0x30};
const FREQ_400: phf::Map<&'static str, u8> = phf_map! {"_" => 0x80, "20" => 0, "25" => 0x10, "50" => 0x20, "110" => 0x30};
const FREQ_800: phf::Map<&'static str, u8> = phf_map! {"_" => 0xC0, "30" => 0, "35" => 0x10, "50" => 0x20, "110" => 0x30};

const ALLOWED_FREQ_BANDWIDTH_COMBINATIONS: phf::Map<u16, phf::Map<&'static str, u8>> = phf_map! {
    100u16 => FREQ_100,
    200u16 => FREQ_200,
    400u16 => FREQ_400,
    800u16 => FREQ_800,
};


pub struct L3G4200D {
    bus: I2c,
    address: u8,
    freq: u16,
    bandwidth: &'static str, 
    combine_filter: f64,
    px: f64,
    py: f64,
    pz: f64,
    cx: f64,
    cy: f64,
    cz: f64,
    buffer_len_in_time: f64,
    data_buffer: Vec<DataPoint>,
    sensitivity: f64,
}

impl L3G4200D {
    pub fn new(address: u8, freq: u16, bandwidth: &'static str, combine_filter: f64) -> L3G4200D {

        match ALLOWED_FREQ_BANDWIDTH_COMBINATIONS.get(&freq) {
            Some(map) =>  if !map.contains_key(&bandwidth) {
                // panic!("Bandwidth {} for frequency {} can be only one of: {}", bandwidth, freq, map);
                panic!("Bandwidth {} for frequency {} is not valid.", bandwidth, freq);
            },
            None => panic!("Fequency can be only one of: 100, 200, 400 or 800; but got {}", freq)
        }
        let mut bus = I2c::with_bus(1).expect("Cannot initialise i2c bus 1");
        bus.set_slave_address(address as u16).expect("Cannot set slave address.");


        let result = L3G4200D {
            bus,
            address, freq, bandwidth, combine_filter,
            px: 0.0, py: 0.0, pz: 0.0,
            cx: 0.0, cy: 0.0, cz: 0.0,
            sensitivity: 0.00875,
            buffer_len_in_time: 10.0,
            data_buffer: vec![DataPoint::empty()]
        };

        result.init_gyro();

        result
    }
    
    fn init_gyro(&self) {
        let selected_freq = ALLOWED_FREQ_BANDWIDTH_COMBINATIONS.get(&self.freq).unwrap();
        let ctrl1 = 0xf + selected_freq.get("_").unwrap() + selected_freq.get(self.bandwidth).unwrap();

        self.bus.smbus_write_byte(CTRL_REG1, ctrl1).expect("Cannot set REG1 on i2c");  // Output data rate 800Hz, freq cut-off 50 (Hz?), normal mode (not power down), all axes (x, y, z) enabled
        self.bus.smbus_write_byte(CTRL_REG2, 0x0).expect("Cannot set REG2 on i2c");
        self.bus.smbus_write_byte(CTRL_REG3, 0x0).expect("Cannot set REG3 on i2c");
        // bus.smbus_write_byte(CTRL_REG4, 0x20);  // Not block (continuous update), LSB @ lower address, FSR 500dps, self test disabled, i2c interface
        // bus.smbus_write_byte(CTRL_REG4, 0x30);  // Not block (continuous update), LSB @ lower address, FSR 2000dps, self test disabled, i2c interface
        self.bus.smbus_write_byte(CTRL_REG4, 0x80).expect("Cannot set REG4 on i2c");  // Not block (continuous update), LSB @ lower address, FSR 500dps, self test disabled, i2c interface
        self.bus.smbus_write_byte(CTRL_REG5, 0x40).expect("Cannot set REG5 on i2c");  // FIFO enabled
        self.bus.smbus_write_byte(FIFO_CTRL_REG, 0x60).expect("Cannot set FIFO_CTRL_REG on i2c");  // FIFO Stream mode

        println!("Initialised L3G4200D i2c device.");
    }

    fn read_data(&self, status: u16, fifo_status: u8, _cut_off_buf_time: f64) -> DataPoint {
//        let data_point = if self.data_buffer.len() > 0 && self.data_buffer[0].time < cut_off_buf_time {
//            let data_point = self.data_buffer[0];
//            self.data_buffer.remove(0);
//            data_point
//        } else {
//            DataPoint::empty()
//        }

        let mut buf = [0u8; 6];
        let _ = self.bus.smbus_block_read(OUT_X_L + 0x80, &mut buf).expect("Cannot read 6 bytes from i2c");

        let dx = LittleEndian::read_i16(&buf[0..1]);
        let dy = LittleEndian::read_i16(&buf[2..3]);
        let dz = LittleEndian::read_i16(&buf[4..5]);

        DataPoint::new(dx, dy, dz, status, fifo_status)
//
//        self.data_buffer.push(data_point);
//        
//        data_point
    }

    pub fn read_deltas(&mut self) -> Vec<DataPoint> {
        let mut result_data: Vec<DataPoint> = vec![];
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let now = since_the_epoch.as_secs_f64();
        let cut_off_buf_time = now - self.buffer_len_in_time;

        let mut waited_for_data = false;
        let mut status: u16 = self.bus.smbus_read_byte(STATUS_REG).expect("Cannot read status from i2c bus") as u16;

        while status & 0xf != 0xf {
            // TODO add check for imdefinite wait
            waited_for_data = true;
            status = self.bus.smbus_read_byte(STATUS_REG).expect("Cannot status byte from i2c bus") as u16;
        }

        if waited_for_data {
            status += 256
        }

        let mut fifo_status: u8 = self.bus.smbus_read_byte(FIFO_SRC_REG).expect("Cannot read fifo_status from i2c bus");

        while fifo_status & 0x1f != 0 {
            // TODO add check for imdefinite wait
            let data_point = self.read_data(status, fifo_status, cut_off_buf_time);
            result_data.push(data_point);
            fifo_status = self.bus.smbus_read_byte(FIFO_SRC_REG).expect("Cannot read fifo_status from i2c bus");
        }

        println!("Got status as {}", status);

        for data_point in &result_data {
            let x = (data_point.dx as f64 - self.cx) * self.sensitivity;
            let y = (data_point.dy as f64 - self.cy) * self.sensitivity;
            let z = (data_point.dz as f64 - self.cz) * self.sensitivity;

            self.px = x * self.combine_filter + (1.0 - self.combine_filter) * self.px;
            self.py = y * self.combine_filter + (1.0 - self.combine_filter) * self.py;
            self.pz = z * self.combine_filter + (1.0 - self.combine_filter) * self.pz;
        }

        result_data
    }
}