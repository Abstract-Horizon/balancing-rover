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


use std::f64::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};

use std::thread;
use std::sync::mpsc;


use crate::telemetry_socket_server::{SocketTelemetryServerBuilder, SocketTelemetryServer};
use crate::telemetry_stream::Storable;
use crate::telemetry_stream::TelemetryStreamDefinition;


use crate::gyro::L3G4200D;
use crate::accel::ADXL345;
use crate::pid::{PID, SIMPLE_DIFFERENCE};


// use crate::telemetry_socket_server::log_with_time;



fn create_logger() -> TelemetryStreamDefinition {
    TelemetryStreamDefinition::new("balance-data", 1,
        vec![
            TelemetryStreamDefinition::signed_word_field("gdx"),
            TelemetryStreamDefinition::signed_word_field("gdy"),
            TelemetryStreamDefinition::signed_word_field("gdz"),
            TelemetryStreamDefinition::double_field("gx"),
            TelemetryStreamDefinition::double_field("gy"),
            TelemetryStreamDefinition::double_field("gz"),
            TelemetryStreamDefinition::unsigned_word_field("status"),
            TelemetryStreamDefinition::unsigned_byte_field("fifo_status"),
            TelemetryStreamDefinition::unsigned_byte_field("data_points"),
            TelemetryStreamDefinition::signed_word_field("adx"),
            TelemetryStreamDefinition::signed_word_field("ady"),
            TelemetryStreamDefinition::signed_word_field("adz"),
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
    accel: ADXL345,
    pid: PID,
    telemetry_server: SocketTelemetryServer,
    logger: TelemetryStreamDefinition,
    combine_factor_gyro: f64,
    freq: f64
}

pub struct BalanceControl {
    balance_sender: mpsc::Sender<bool>,
    balance_thread: thread::JoinHandle<()>
}

impl BalanceControl {
    pub fn stop(self) {
        let _ = self.balance_sender.send(true);
        let _ = self.balance_thread.join();
    }
}

impl Balance {
    pub fn new() -> Balance {
        let mut socket_server_builder = SocketTelemetryServerBuilder::new();
        let logger = socket_server_builder.register_stream(create_logger());

        let telemetry_server = socket_server_builder.create(1860);

        let freq: u16 = 200;

        Balance {
            gyro: L3G4200D::new(0x69, freq, "50", 0.3),
            accel: ADXL345::new(0x53, freq, 0.5),
            pid: PID::new(0.75, 0.2, 0.05, 1.0, 0.0001, 1.0, 100.0, SIMPLE_DIFFERENCE),
            telemetry_server,
            logger,
            combine_factor_gyro: 0.95,
            freq: 200.0
        }
    }

    pub fn start(self) -> BalanceControl {
        let (sender, receiver) = mpsc::channel();

        BalanceControl {
            balance_sender: sender,
            balance_thread: thread::spawn(move || {
                self.run_loop(receiver);
            })
        }
    }

    fn run_loop(self, receiver: mpsc::Receiver<bool>) {
        let mut gyro = self.gyro;
        let mut accel = self.accel;
        let mut pid = self.pid;
        let mut cx: f64 = 0.0;
        let mut cy: f64 = 0.0;
        let mut cz: f64 = 0.0;
        let mut pid_time: f64 = 0.0;
        // let mut delta_time: f64 = 0.0;
        // let mut control: f64 = 0.0;
        let bump: f64 = 0.0;

        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let mut last_time = since_the_epoch.as_secs_f64();


        loop {
            match receiver.try_recv() {
                Ok(_) => break,
                _ => {}
            };
            let gyro_data_points = gyro.read_deltas();
            let gyro_data_point_len = gyro_data_points.len();

            let accel_data_point = accel.read();

            let accel_pitch = (accel_data_point.z.atan2((accel_data_point.x * accel_data_point.x + accel_data_point.y * accel_data_point.y).sqrt()) * 180.0) / PI;
            let accel_roll = (accel_data_point.x.atan2((accel_data_point.z * accel_data_point.z + accel_data_point.y * accel_data_point.y).sqrt()) * 180.0) / PI;
            let accel_yav = (accel_data_point.y.atan2((accel_data_point.z * accel_data_point.z + accel_data_point.x * accel_data_point.x).sqrt()) * 180.0) / PI;

            for gyro_data_point in gyro_data_points {

                cx = (cx + gyro.px / gyro.freq) * self.combine_factor_gyro + accel_yav * (1.0 - self.combine_factor_gyro);
                cy = (cy + gyro.py / gyro.freq) * self.combine_factor_gyro + accel_pitch * (1.0 - self.combine_factor_gyro);
                cz = (cz + gyro.pz / gyro.freq) * self.combine_factor_gyro + accel_roll * (1.0 - self.combine_factor_gyro);

                let start = SystemTime::now();
                let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
                let now = since_the_epoch.as_secs_f64();

                let delta_time = now - last_time;
                last_time = now;

                let output = pid.process(pid_time, 0.0, (cy * PI / 90.0).sin() * 2.0);

                let control = output;

                pid_time += 1.0 / self.freq;


                log_with_time!(
                    self.telemetry_server, self.logger,
                    gyro_data_point.dx, gyro_data_point.dy, gyro_data_point.dz,
                    gyro.px, gyro.py, gyro.pz,
                    gyro_data_point.status, gyro_data_point.fifo_status, gyro_data_point_len as u8,
                    accel_data_point.raw_x, accel_data_point.raw_y, accel_data_point.raw_z,
                    accel_data_point.x, accel_data_point.y, accel_data_point.z,
                    accel_pitch, accel_roll, accel_yav,
                    cx, cy, cz,
                    pid.p, pid.i, pid.d,
                    pid.p * pid.kp, pid.i * pid.ki, pid.d *pid.kd,
                    delta_time, output, control, bump);
            }
        }

        println!("Trying to kill threads...");
        self.telemetry_server.stop();
        println!("Finishing!");
    }
}
