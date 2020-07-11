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

use dma_gpio::pi::{BoardBuilder, Board};

const LEFT_PWM_PIN_NO: u8 = 20;
const LEFT_IN1_PIN_NO: u8 = 6;
const LEFT_IN2_PIN_NO: u8 = 5;
const RIGHT_PWM_PIN_NO: u8 = 26;
const RIGHT_IN1_PIN_NO: u8 = 19;
const RIGHT_IN2_PIN_NO: u8 = 13;



fn sanitise_speed(speed: f32) -> (f32, i32) {
    let mut speed = speed;

    if speed > 0.0001 {
        if speed > 1.0 {
            speed = 1.0;
        } else if speed < 0.01 {
            speed = 0.0;
        }
        (speed, 1)
    } else if speed < -0.00001 {
        speed *= -1.0;
        if speed > 1.0 {
            speed = 1.0;
        } else if speed < 0.01 {
            speed = 0.0;
        }
        (speed, -1)
    } else {
        (0.0, 0)
    }
}


pub struct Motors {
    left_in1_pin: OutputPin,
    left_in2_pin: OutputPin,
    left_last_direction: i32,
    right_in1_pin: OutputPin,
    right_in2_pin: OutputPin,
    right_last_direction: i32,
    board: Board
}

impl Motors {
    pub fn new() -> Motors {

        let mut motors = Motors {
            left_in1_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get left in1 pin {}", LEFT_IN1_PIN_NO))
                .get(LEFT_IN1_PIN_NO).unwrap_or_else(|_| panic!("Cannot get left in2 pin {}", LEFT_IN1_PIN_NO))
                .into_output(),
            left_in2_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get left in2 pin {}", LEFT_IN2_PIN_NO))
                .get(LEFT_IN2_PIN_NO).unwrap_or_else(|_| panic!("Cannot get left in2 pin {}", LEFT_IN2_PIN_NO))
                .into_output(),
            left_last_direction: 0,
            right_in1_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get right in1 pin {}", RIGHT_IN1_PIN_NO))
                .get(RIGHT_IN1_PIN_NO).unwrap_or_else(|_| panic!("Cannot get right in1 pin {}", RIGHT_IN1_PIN_NO))
                .into_output(),
            right_in2_pin: Gpio::new().unwrap_or_else(|_| panic!("Cannot get right in2 pin {}", RIGHT_IN2_PIN_NO))
                .get(RIGHT_IN2_PIN_NO).unwrap_or_else(|_| panic!("Cannot get right in2 pin {}", RIGHT_IN2_PIN_NO))
                .into_output(),
            right_last_direction: 0,
            board: BoardBuilder::new()
                .divide_pwm(1250)
                .set_cycle_time(200)
                .set_sample_delay(2)
                .build_with_pins(vec![LEFT_PWM_PIN_NO, RIGHT_PWM_PIN_NO]).unwrap_or_else(|_| panic!("Cannot get setup PWM for pins {} and {}", LEFT_PWM_PIN_NO, RIGHT_PWM_PIN_NO))
        };

        motors.stop_all();

        motors
    }

    pub fn stop_all(&mut self) {
        self.left_speed(0.0);
        self.right_speed(0.0);

//        self.left_in1_pin.set_high();
//        self.left_in2_pin.set_high();
//        self.right_in1_pin.set_high();
//        self.right_in2_pin.set_high();
//        self.board.set_all_pwm(0.0).unwrap();
    }


    pub fn left_speed(&mut self, speed: f32) {
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

        self.board.set_pwm(LEFT_PWM_PIN_NO, speed).unwrap_or_else(|_| panic!("Cannot get set PWM for pin {}", LEFT_PWM_PIN_NO));
    }

    pub fn right_speed(&mut self, speed: f32) {
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

        self.board.set_pwm(RIGHT_PWM_PIN_NO, speed).unwrap_or_else(|_| panic!("Cannot get set PWM for pin {}", LEFT_PWM_PIN_NO));
    }
}
