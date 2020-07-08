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


#![feature(macro_rules)]

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rppal::gpio::Gpio;
use rppal::i2c::I2c;
use rppal::pwm::{Channel, Pwm};

use crate::telemetry_socket_server;
use crate::telemetry_socket_server::{SocketTelemetryServerBuilder, SocketTelemetryServer};
use crate::telemetry_stream::Storable;
use crate::telemetry_stream::TelemetryStreamDefinition;


use crate::gyro::L3G4200D;

// use crate::telemetry_socket_server::log_with_time;



fn create_logger() -> TelemetryStreamDefinition {
    TelemetryStreamDefinition::new("balance-data", 1,
        vec![
            TelemetryStreamDefinition::signed_integer_field("gdx"),
            TelemetryStreamDefinition::signed_integer_field("gdy"),
            TelemetryStreamDefinition::signed_integer_field("gdz"),
            TelemetryStreamDefinition::double_field("gx"),
            TelemetryStreamDefinition::double_field("gy"),
            TelemetryStreamDefinition::double_field("gz"),
            TelemetryStreamDefinition::unsigned_word_field("status"),
            TelemetryStreamDefinition::unsigned_byte_field("fifo_status"),
            TelemetryStreamDefinition::unsigned_byte_field("data_points"),
            TelemetryStreamDefinition::signed_integer_field("adx"),
            TelemetryStreamDefinition::signed_integer_field("ady"),
            TelemetryStreamDefinition::signed_integer_field("adz"),
            TelemetryStreamDefinition::double_field("ax"),
            TelemetryStreamDefinition::double_field("ay"),
            TelemetryStreamDefinition::double_field("az"),
            TelemetryStreamDefinition::double_field("apitch"),
            TelemetryStreamDefinition::double_field("aroll"),
            TelemetryStreamDefinition::double_field("ayaw"),
            TelemetryStreamDefinition::double_field("cx"),
            TelemetryStreamDefinition::double_field("cy"),
            TelemetryStreamDefinition::double_field("cz"),
            TelemetryStreamDefinition::double_field("pi_p"),
            TelemetryStreamDefinition::double_field("pi_i"),
            TelemetryStreamDefinition::double_field("pi_d"),
            TelemetryStreamDefinition::double_field("pi_pg"),
            TelemetryStreamDefinition::double_field("pi_ig"),
            TelemetryStreamDefinition::double_field("pi_dg"),
            TelemetryStreamDefinition::double_field("pi_dt"),
            TelemetryStreamDefinition::double_field("pi_o"),
            TelemetryStreamDefinition::double_field("out"),
            TelemetryStreamDefinition::double_field("bump"),
        ]
    )
}


pub struct Balance {
    gyro: L3G4200D,
    telemetry_server: SocketTelemetryServer,
    logger: TelemetryStreamDefinition,
}


impl Balance {
    pub fn new() -> Balance {
        let mut socket_server_builder = SocketTelemetryServerBuilder::new();
        let logger = socket_server_builder.register_stream(create_logger());

        let telemetry_server = socket_server_builder.create(1860);

        Balance {
            gyro: L3G4200D::new(0x69, 200, "50", 0.3),
            telemetry_server,
            logger
        }
    }

    pub fn run_loop(self) {
        let mut gyro = self.gyro;

        let s1 = String::from("1234567890");
        let s2 = String::from("0123456789ABCDEF");
        let bytes = s2.as_bytes();

        for i in 1..250 {

            let _ = gyro.read_deltas();

            log_with_time!(self.telemetry_server, self.logger,
                            i as u32, i as u32, i as u32,
                            i as f64, i as f64, i as f64,
                            i as u16, i as u8, i as u8,
                            i as i32, i as i32, i as i32, 
                            i as f64, i as f64, i as f64,
                            i as f64, i as f64, i as f64,
                            i as f64, i as f64, i as f64,
                            i as f64, i as f64, i as f64,
                            i as f64, i as f64, i as f64,
                            i as f64, i as f64, i as f64, i as f64);
            println!("Sending {} to log, stream {}", i, self.logger.name());

            thread::sleep(Duration::from_millis(333));
        }

        println!("Trying to kill threads...");
        self.telemetry_server.stop();
        println!("Finishing!");
    }
}
