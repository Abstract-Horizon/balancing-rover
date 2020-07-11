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


use byteorder::{ByteOrder, LittleEndian};
use phf::phf_map;

use rppal::i2c::I2c;

#[allow(dead_code)]
const EARTH_GRAVITY_MS2: f64 = 9.80665;
// const SCALE_MULTIPLIER: f64 = 0.004;
const SCALE_MULTIPLIER: f64 = 0.00390625;

const DATA_FORMAT: u8 = 0x31;
const BW_RATE: u8 = 0x2C;
const POWER_CTL: u8 = 0x2D;

const BW_RATE_1600HZ: u8 = 0x0F;
const BW_RATE_800HZ: u8 = 0x0E;
const BW_RATE_400HZ: u8 = 0x0D;
const BW_RATE_200HZ: u8 = 0x0C;
const BW_RATE_100HZ: u8 = 0x0B;
const BW_RATE_50HZ: u8 = 0x0A;
const BW_RATE_25HZ: u8 = 0x09;

#[allow(dead_code)]
const RANGE_2G: u8 = 0x00;
#[allow(dead_code)]
const RANGE_4G: u8 = 0x01;
#[allow(dead_code)]
const RANGE_8G: u8 = 0x02;
const RANGE_16G: u8 = 0x03;

const MEASURE: u8 = 0x08;
const AXES_DATA: u8 = 0x32;


// #[derive(Clone)]
pub struct DataPoint {
    pub raw_x: i16,
    pub raw_y: i16,
    pub raw_z: i16,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl DataPoint {
    pub fn new(raw_x: i16, raw_y: i16, raw_z: i16, x: f64, y: f64, z: f64) -> DataPoint {
        DataPoint { raw_x, raw_y, raw_z, x, y, z }
    }
}

const ALLOWED_FREQUENCIES: phf::Map<u16, u8> = phf_map! {
    1600u16 => BW_RATE_1600HZ,
    800u16 => BW_RATE_800HZ,
    400u16 => BW_RATE_400HZ,
    200u16 => BW_RATE_200HZ,
    100u16 => BW_RATE_100HZ,
    50u16 => BW_RATE_50HZ,
    25u16 => BW_RATE_25HZ
};


pub struct ADXL345 {
    bus: I2c,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub x_offset: f64,
    pub y_offset: f64,
    pub z_offset: f64,
    pub combine_filter: f64,
}

impl ADXL345 {
    pub fn new(address: u8, freq: u16, combine_filter: f64) -> ADXL345 {

        let mut bus = I2c::with_bus(1).expect("Cannot initialise i2c bus 1");
        bus.set_slave_address(address as u16).expect("Cannot set slave address.");

        let adxl345 = ADXL345 {
            bus,
            x: 0.0, y: 0.0, z: 0.0, x_offset: 0.0, y_offset: 0.0, z_offset: 0.0,
            combine_filter,
        };

        match ALLOWED_FREQUENCIES.get(&freq) {
            Some(rate) => adxl345.set_bandwidth_rate(*rate),
            None => panic!("Unexpected freqency {}", freq)
        }

        adxl345.set_range(RANGE_16G);

        adxl345.enable_measurement();

        adxl345
    }

    pub fn set_bandwidth_rate(&self, rate_flag: u8) {
        self.bus.smbus_write_byte(BW_RATE, rate_flag).expect("Cannot set BW_RATE on i2c");
    }

    pub fn set_range(&self, range_flag: u8) {
        let mut value = self.bus.smbus_read_byte(DATA_FORMAT).expect("Cannot read DATA_FORMAT byte from i2c");

        value &= !0x0F;
        value |= range_flag;
        value |= 0x08; // FULL RES

        self.bus.smbus_write_byte(DATA_FORMAT, value).expect("Cannot set BW_RATE on i2c");
    }

    pub fn enable_measurement(&self) {
        self.bus.smbus_write_byte(POWER_CTL, MEASURE).expect("Cannot set BW_RATE on i2c");
    }

    pub fn read(&mut self) -> DataPoint {

        let command: [u8; 1] = [AXES_DATA];
        let mut buf = [0u8; 6];
        let _ = self.bus.write_read(&command, &mut buf).expect("Cannot read 6 bytes from i2c");

        let raw_x = LittleEndian::read_i16(&buf[0..2]);
        let raw_y = LittleEndian::read_i16(&buf[2..4]);
        let raw_z = LittleEndian::read_i16(&buf[4..6]);

        let invert_combine_filter = 1.0 - self.combine_filter;
        self.x = (raw_x as f64 * SCALE_MULTIPLIER - self.x_offset) * self.combine_filter + self.x  * invert_combine_filter;
        self.y = (raw_y as f64 * SCALE_MULTIPLIER - self.y_offset) * self.combine_filter + self.y  * invert_combine_filter;
        self.z = (raw_z as f64 * SCALE_MULTIPLIER - self.z_offset) * self.combine_filter + self.z  * invert_combine_filter;

        DataPoint::new(raw_x, raw_y, raw_z, self.x, self.y, self.z)
    }
}


