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

// #![feature(macro_rules)]


mod telemetry_stream;

// #[macro_use]
mod telemetry_socket_server;

mod pid;
mod motors;
mod balance;
mod gyro;
mod accel;

use balance::{Balance, ConfigData};


use std::time::Duration;
use std::thread;



fn main() {
    let config_data = ConfigData::new();
    let balance = Balance::new(config_data);

    let balance_control = balance.start();

    thread::sleep(Duration::from_secs(3600));

    balance_control.stop();
}
