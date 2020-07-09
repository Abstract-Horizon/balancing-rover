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


use crate::motors::Motors;
use crate::gyro::L3G4200D;
use crate::accel::ADXL345;
use crate::pid::{PID, SIMPLE_DIFFERENCE};


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


#[derive(Clone, Copy)]
pub struct ConfigData {
    freq: u16,
    combine_gyro_accel_factor: f64,
    combine_gyro_factor: f64,
    combine_accel_factor: f64,
    pid_kp: f64, pid_ki: f64, pid_kd: f64, pid_gain: f64,
    dead_band: f64, i_gain_scale: f64, d_gain_scale: f64,
    max_degree: f64,
    start_degree: f64,
}

impl ConfigData {
    pub fn new() -> ConfigData {
        ConfigData {
            freq: 200,
            combine_gyro_accel_factor: 0.95,
            combine_gyro_factor: 0.3,
            combine_accel_factor: 0.5,
            pid_kp: 0.75,
            pid_ki: 0.2,
            pid_kd: 0.05,
            pid_gain: 1.0,
            dead_band: 0.0001,
            i_gain_scale: 1.0,
            d_gain_scale: 100.0,
            max_degree: 45.0,
            start_degree: 4.0,
        }
    }
}


pub struct Balance {
    telemetry_server: SocketTelemetryServer,
    logger: TelemetryStreamDefinition,
    config_data: ConfigData,
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

// #[derive(Clone, Copy)]
enum State {
    #[allow(dead_code)]
    Stopped,
    WaitingForReady,
    Balancing
}

impl Balance {
    pub fn new(config_data: ConfigData) -> Balance {
        let mut socket_server_builder = SocketTelemetryServerBuilder::new();
        let logger = socket_server_builder.register_stream(create_logger());

        let telemetry_server = socket_server_builder.create(1860);

        Balance {
            telemetry_server,
            logger,
            config_data: config_data.clone(),
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
        let config_data = self.config_data;
        let mut gyro = L3G4200D::new(0x69, config_data.freq, "50", config_data.combine_gyro_factor);
        let mut accel = ADXL345::new(0x53, config_data.freq, config_data.combine_accel_factor);
        let mut pid = PID::new(
            config_data.pid_kp, config_data.pid_ki, config_data.pid_kd,
            config_data.pid_gain, config_data.dead_band,
            config_data.i_gain_scale, config_data.d_gain_scale, SIMPLE_DIFFERENCE);

        let mut motors = Motors::new();
        let mut cx: f64 = 0.0;
        let mut cy: f64 = 0.0;
        let mut cz: f64 = 0.0;
        let mut pid_time: f64 = 0.0;
        let bump: f64 = 0.0;

        let freq_f64 = config_data.freq as f64;

        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let mut last_time = since_the_epoch.as_secs_f64();

        let mut state = State::WaitingForReady;

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

                let combine_gyro_accel_factor = config_data.combine_gyro_accel_factor;
                let invert_combine_gyro_accel_factor = 1.0 - combine_gyro_accel_factor;
                cx = (cx + gyro.px / gyro.freq) * combine_gyro_accel_factor + accel_yav * invert_combine_gyro_accel_factor;
                cy = (cy + gyro.py / gyro.freq) * combine_gyro_accel_factor + accel_pitch * invert_combine_gyro_accel_factor;
                cz = (cz + gyro.pz / gyro.freq) * combine_gyro_accel_factor + accel_roll * invert_combine_gyro_accel_factor;

                let start = SystemTime::now();
                let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
                let now = since_the_epoch.as_secs_f64();

                let delta_time = now - last_time;
                last_time = now;

                let output = pid.process(pid_time, 0.0, (cy * PI / 90.0).sin() * 2.0);

                let control = output;

                pid_time += 1.0 / freq_f64;

                match state {
                    State::Stopped => {
                    },
                    State::WaitingForReady => {
                        if -config_data.start_degree < cy && cy < config_data.start_degree {
                            state = State::Balancing;
                        }
                    },
                    State::Balancing => {
                        if cy < -config_data.max_degree || cy > config_data.max_degree {
                            state = State::WaitingForReady;
                            motors.left_speed(0.0);
                            motors.right_speed(0.0);
                            println!("*** Got over {} def stopping!", config_data.max_degree);
                        } else {
                            motors.left_speed(control as f32);
                            motors.right_speed(control as f32);
                        }
                    }
                }

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
