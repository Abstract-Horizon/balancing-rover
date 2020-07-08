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

#[allow(non_snake_case)]
pub fn SIMPLE_DIFFERENCE(x: f64, y: f64) -> f64 { x - y }

pub struct PID {
    pub set_point: f64,
    pub p: f64,
    pub i: f64,
    pub d: f64,
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub kg: f64,
    pub i_gain_scale: f64,
    pub d_gain_scale: f64,
    pub dead_band: f64,
    pub last_error: f64,
    pub last_time: f64,
    pub last_output: f64,
    pub last_delta: f64,
    first: bool,
    difference: fn(f64, f64) -> f64,
}

impl PID {
    pub fn new(
        p_gain: f64, i_gain: f64, d_gain: f64, gain: f64,
        dead_band: f64, i_gain_scale: f64, d_gain_scale: f64,
        difference: fn(f64, f64) -> f64) -> PID {

        PID {
            set_point: 0.0,
            p: 0.0, i: 0.0, d: 0.0,
            kp: p_gain, ki: i_gain, kd: d_gain, kg: gain,
            i_gain_scale, d_gain_scale,
            dead_band,
            last_error: 0.0,
            last_time: 0.0,
            last_output: 0.0,
            last_delta: 0.0,
            first: true,
            difference
        }
    }

    pub fn process(&mut self, time:f64, set_point: f64, current: f64) -> f64 {

        let mut error = (self.difference)(set_point, current);

        if error.abs() <= self.dead_band {
            error = 0.0;
        }

        if self.first {
            self.first = false;
            self.set_point = set_point;
            self.last_error = error;
            self.last_time = time;

            0.0
        } else {
            let delta_time = time - self.last_time;

            self.p = error;
            if (self.last_error < 0.0 && 0.0 < error) || (self.last_error > 0.0 && 0.0 > error) {
                self.i = 0.0
            } else if error.abs() <= 0.01 {
                self.i = 0.0;
            } else {
                self.i += error * delta_time * self.i_gain_scale
            }

            if delta_time > 0.0 {
                self.d = (error - self.last_error) / (delta_time * self.d_gain_scale);
            }

            let mut output = self.p * self.kp + self.i * self.ki + self.d * self.kd;

            output *= self.kg;

            self.set_point = set_point;
            self.last_output = output;
            self.last_error = error;
            self.last_time = time;
            self.last_delta = delta_time;

            output
        }
    }
}
