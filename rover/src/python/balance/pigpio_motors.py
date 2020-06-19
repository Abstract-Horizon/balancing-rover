################################################################################
# Copyright (C) 2020 Abstract Horizon
# All rights reserved. This program and the accompanying materials
# are made available under the terms of the Apache License v2.0
# which accompanies this distribution, and is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
#  Contributors:
#    Daniel Sendula - initial API and implementation
#
#################################################################################

import pigpio


gpios = pigpio.pi()


class PIGPIOMotors:
    def __init__(self, left_pwm_pin=20, left_in1_pin=5, left_in2_pin=6, right_pwm_pin=26, right_in1_pin=13, right_in2_pin=19, pwm_freq=8000):
        self.left_pwm_pin = left_pwm_pin
        self.left_in1_pin = left_in1_pin
        self.left_in2_pin = left_in2_pin

        self.right_pwm_pin = right_pwm_pin
        self.right_in1_pin = right_in1_pin
        self.right_in2_pin = right_in2_pin

        self.pwm_freq = pwm_freq

        self.left_pwm = None
        self.right_pwm = None
        self.left_last_dir = 0
        self.right_last_dir = 0

    def init(self):

        gpios.write(self.left_pwm_pin, 0)
        gpios.write(self.left_in1_pin, 1)
        gpios.write(self.left_in2_pin, 1)
        gpios.write(self.right_pwm_pin, 0)
        gpios.write(self.right_in1_pin, 1)
        gpios.write(self.right_in2_pin, 1)

        gpios.set_PWM_frequency(26, self.pwm_freq)
        gpios.set_PWM_range(self.left_pwm_pin, 100)
        gpios.set_PWM_range(self.right_pwm_pin, 100)

        gpios.set_PWM_dutycycle(self.left_pwm_pin, 0)
        gpios.set_PWM_dutycycle(self.right_pwm_pin, 0)

    # noinspection PyMethodMayBeStatic
    def sanitise_speed(self, speed):
        if speed > 0.0001:
            direction = 1
            speed *= 100.0
            if speed > 100.0:
                speed = 100.0
            elif speed < 1.0:
                speed = 0.0
        elif speed < -0.00001:
            direction = -1
            speed *= -100.0
            if speed > 100.0:
                speed = 100.0
            elif speed < 1.0:
                speed = 0.0
        else:
            direction = 0
            speed = 0.0
        return speed, direction

    def left_speed(self, speed):
        speed, direction = self.sanitise_speed(speed)

        if self.left_last_dir != direction:
            self.left_last_dir = direction
            if direction == 1:
                gpios.write(self.left_in1_pin, 0)
                gpios.write(self.left_in2_pin, 1)
                #
                # GPIO.output(self.left_in2_pin, 1)
                # GPIO.output(self.left_in1_pin, 0)
            elif direction == -1:
                gpios.write(self.left_in1_pin, 1)
                gpios.write(self.left_in2_pin, 0)
                # GPIO.output(self.left_in1_pin, 1)
                # GPIO.output(self.left_in2_pin, 0)
            else:
                gpios.write(self.left_in1_pin, 1)
                gpios.write(self.left_in2_pin, 1)
        try:
            gpios.set_PWM_dutycycle(self.left_pwm_pin, speed)
        except Exception as ex:
            print(f"Tried left speed of {speed} and failed. {ex}")

    def right_speed(self, speed):
        speed, direction = self.sanitise_speed(speed)

        if self.right_last_dir != direction:
            self.right_last_dir = direction
            if direction == 1:
                gpios.write(self.right_in1_pin, 1)
                gpios.write(self.right_in2_pin, 0)
                #
                # GPIO.output(self.right_in1_pin, 1)
                # GPIO.output(self.right_in2_pin, 0)
            elif direction == -1:
                gpios.write(self.right_in1_pin, 0)
                gpios.write(self.right_in2_pin, 1)
                # GPIO.output(self.right_in2_pin, 1)
                # GPIO.output(self.right_in1_pin, 0)
            else:
                gpios.write(self.right_in1_pin, 1)
                gpios.write(self.right_in2_pin, 1)

        try:
            gpios.set_PWM_dutycycle(self.right_pwm_pin, speed)
        except Exception as ex:
            print(f"Tried right speed of {speed} and failed. {ex}")
