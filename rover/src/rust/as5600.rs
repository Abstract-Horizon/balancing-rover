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

use byteorder::{ByteOrder, BigEndian};
use rppal::i2c::I2c;


const _STATUS_ERROR_I2C_WRITE: u8 = 1;
const _STATUS_ERROR_I2C_READ: u8 = 2;
const _STATUS_ERROR_MOTOR_OVERHEAT: u8 = 4;
const _STATUS_ERROR_MAGNET_HIGH: u8 = 8;
const _STATUS_ERROR_MAGNET_LOW: u8 = 16;
const _STATUS_ERROR_MAGNET_NOT_DETECTED: u8 = 32;
const _STATUS_ERROR_RX_FAILED: u8 = 64;
const _STATUS_ERROR_TX_FAILED: u8 = 128;


pub struct AS5600 {
    bus: I2c,
    dir: i8,
    pub deg: f64,
    pub last_deg: f64,
    pub status: u8
}

impl AS5600 {
    pub fn new(bus: u8, dir: i8) -> AS5600 {
        let mut bus = I2c::with_bus(bus).unwrap_or_else(|_| panic!("Cannot initialise i2c bus {}", bus));
        bus.set_slave_address(0x36).expect("Cannot set slave address to 0x36.");
        
        AS5600 {
            bus,
            dir,
            deg: 0.0,
            last_deg: 0.0,
            status: 0
        }
    }

    pub fn read(&mut self) -> f64 {
        let mut buf = [0u8; 5];
        let command: [u8; 1] = [0x0B];
        let _ = self.bus.write_read(&command, &mut buf).expect("AS5600: Cannot read 2 bytes from i2c");

        self.last_deg = self.deg;

        if self.dir < 0 {
            self.deg = (4096 - BigEndian::read_i16(&buf[3..5])) as f64 * 360.0 / 4096.0;
        } else {
            self.deg = BigEndian::read_i16(&buf[3..5]) as f64 * 360.0 / 4096.0;
        }
        self.status  = buf[0] & 0b00111000 | _STATUS_ERROR_MAGNET_NOT_DETECTED;
        
        self.deg
    }
}
