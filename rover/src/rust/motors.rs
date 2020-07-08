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

use rppal::gpio::{Gpio, OutputPin};



fn sanitise_speed(speed: f64) -> (f64, i32) {
    let mut speed = speed;

    if speed > 0.0001 {
        speed *= 100.0;
        if speed > 100.0 {
            speed = 100.0;
        } else if speed < 1.0 {
            speed = 0.0;
        }
        (speed, 1)
    } else if speed < -0.00001 {
        speed *= -100.0;
        if speed > 100.0 {
            speed = 100.0;
        } else if speed < 1.0 {
            speed = 0.0;
        }
        (speed, -1)
    } else {
        (0.0, 0)
    }
}


pub struct Motors {
    left_pwm_pin: OutputPin,
    left_in1_pin: OutputPin,
    left_in2_pin: OutputPin,
    left_last_direction: i32,
    right_pwm_pin: OutputPin,
    right_in1_pin: OutputPin,
    right_in2_pin: OutputPin,
    right_last_direction: i32,
}

impl Motors {
    pub fn new() -> Motors {
        let left_pwm_pin_no = 20;
        let left_in1_pin_no = 6;
        let left_in2_pin_no = 5;
        let right_pwm_pin_no = 26;
        let right_in1_pin_no = 13;
        let right_in2_pin_no = 19;
        let pwm_freq = 8000;

        let mut motors = Motors {
            left_pwm_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get left PWM pin {}", left_pwm_pin_no))
                .get(left_pwm_pin_no).unwrap_or_else(|_| panic!("Cannot get left PWM pin {}", left_pwm_pin_no))
                .into_output(),
            left_in1_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get left in1 pin {}", left_in1_pin_no))
                .get(left_in1_pin_no).unwrap_or_else(|_| panic!("Cannot get left in2 pin {}", left_in1_pin_no))
                .into_output(),
            left_in2_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get left in2 pin {}", left_in2_pin_no))
                .get(left_in2_pin_no).unwrap_or_else(|_| panic!("Cannot get left in2 pin {}", left_in2_pin_no))
                .into_output(),
            left_last_direction: 0,
            right_pwm_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get right PWM pin {}", right_pwm_pin_no))
                .get(right_pwm_pin_no).unwrap_or_else(|_| panic!("Cannot get right PWM pin {}", right_pwm_pin_no))
                .into_output(),
            right_in1_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get right in1 pin {}", right_in1_pin_no))
                .get(right_in1_pin_no).unwrap_or_else(|_| panic!("Cannot get right in1 pin {}", right_in1_pin_no))
                .into_output(),
            right_in2_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get right in2 pin {}", right_in2_pin_no))
                .get(right_in2_pin_no).unwrap_or_else(|_| panic!("Cannot get right in2 pin {}", right_in2_pin_no))
                .into_output(),
            right_last_direction: 0,
        };
        
        motors.stop_all();
        
        motors
    }
    
    pub fn stop_all(&mut self) {
        self.left_pwm_pin.set_high();
        self.left_in1_pin.set_high();
        self.left_in2_pin.set_high();
        self.right_pwm_pin.set_high();
        self.right_in1_pin.set_high();
        self.right_in2_pin.set_high();
    }


    pub fn left_speed(&mut self, speed: f64) {
        let (speed, direction) = sanitise_speed(speed);

        if self.left_last_direction != direction {
            self.left_last_direction = direction;
            if direction == 1 {
                self.left_in1_pin.set_low();
                self.left_in2_pin.set_high();
            } else if direction == -1 {
                self.left_in1_pin.set_high();
                self.left_in2_pin.set_low();
            } else {
                self.left_in1_pin.set_high();
                self.left_in2_pin.set_high();
            }
        }
//        try:
//            gpios.set_PWM_dutycycle(self.left_pwm_pin, speed)
//        except Exception as ex:
//            print(f"Tried left speed of {speed} and failed. {ex}")
    }

    pub fn right_speed(&mut self, speed: f64) {
        let (speed, direction) = sanitise_speed(speed);

        if self.right_last_direction != direction {
            self.right_last_direction = direction;
            if direction == 1 {
                self.right_in1_pin.set_low();
                self.right_in2_pin.set_high();
            } else if direction == -1 {
                self.right_in1_pin.set_high();
                self.right_in2_pin.set_low();
            } else {
                self.right_in1_pin.set_high();
                self.right_in2_pin.set_high();
            }
        }
    }
}
