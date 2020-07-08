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


mod telemetry_stream;

#[macro_use]
mod telemetry_socket_server;

mod balance;
mod gyro;




use balance::Balance;



fn main() {
    let balance = Balance::new();

    balance.run_loop()
}
