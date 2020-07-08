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

import traceback
import telemetry
import threading
import time
import math

from balance.accel import ADXL345
from balance.gyro import L3G4200D
from balance.pigpio_motors import PIGPIOMotors
from balance.pid import PID


class Balance:
    _DEFAULT_PID = {'p': 0.75, 'i': 0.2, 'd': 0.05, 'g': 1.0}

    STATE_STOPPED = 0
    STATE_WAITING_FOR_READY = 1
    STATE_BALANCING = 2

    def __init__(self,
                 freq=200,
                 gyro_bandwidth=25,
                 gyro_filter=0.3,
                 accel_filter=0.5,
                 combine_factor_gyro=0.95,
                 max_deg=45.0,
                 bump_threshold=0.10,
                 bump_delay=0.2,
                 bump_gain=1.0,
                 bump_step=0.2,
                 bump_len=0.3,
                 pid_inner: PID = None,
                 pid_outer: PID = None):

        self._debug = False

        self.freq = freq

        self.gyro = L3G4200D(freq=freq, bandwidth=gyro_bandwidth, combine_filter=gyro_filter)
        self.accel = ADXL345(freq=freq, combine_filter=accel_filter)
        self.motors = PIGPIOMotors()
        self.combine_factor_gyro = combine_factor_gyro
        self.cx = 0.0
        self.cy = 0.0
        self.cz = 0.0

        self.max_deg = max_deg
        self.bump_threshold=bump_threshold
        self.bump_delay=bump_delay
        self.bump_gain=bump_gain
        self.bump_step=bump_step
        self.bump_len=bump_len
        self.bump_found=None
        self.bump_in_place=None
        # self.expo = expo
        # self.output_dead_band = 0.001
        # self.output_offset = 0.05

        self.telemetry_port = 1860

        self._thread = None
        self._logger = None
        self.telemetry_server = None
        self.state = self.STATE_STOPPED
        self.pid_inner: PID = pid_inner if pid_inner is not None else PID().update_gains_from_map(self._DEFAULT_PID)
        self.pid_outer: PID = pid_outer if pid_outer is not None else PID().update_gains_from_map(self._DEFAULT_PID)

        print("Starting balance with following config values:")
        print(f"  Freq {self.freq}")
        print(f"  Gyro  freq={freq}, bandwidth={gyro_bandwidth} and filter={gyro_filter}")
        print(f"  Accel freq={freq} and filter={accel_filter}")
        print(f"  Combine factor of  {combine_factor_gyro}")
        print(f"  Bump threshold={self.bump_threshold}, delay={self.bump_delay}, gain={self.bump_gain}, step={self.bump_gain}, len={self.bump_len}")

        # print(f"  expo {self.expo}")
        print(f"  PID inner {pid_inner}")
        print(f"  PID outer {pid_outer}")

    def run_loop(self):
        print("    started loop thread.")
        had_exception = False

        output = 0
        last_state = self.state
        last_time = time.time()
        delta_time = 0
        pid_time = 0
        # step_factor = 10
        # amplitude = 13  # 0.02  0.08
        # longitude = 100  # 10/s

        bump = 0
        control = 0
        last_cy = None

        while True:
            try:
                gyro_data_points = self.gyro.read_deltas()
                accel_data_point = self.accel.read()

                accel_dx, accel_dy, accel_dz = accel_data_point.get_raw_data()
                accel_x, accel_y, accel_z = accel_data_point.get_data()

                accel_pitch = (math.atan2(accel_z, math.sqrt(accel_x * accel_x + accel_y * accel_y)) * 180.0) / math.pi
                accel_roll = (math.atan2(accel_x, (math.sqrt(accel_z * accel_z + accel_y * accel_y))) * 180.0) / math.pi
                accel_yav = (math.atan2(accel_y, (math.sqrt(accel_z * accel_z + accel_x * accel_x))) * 180.0) / math.pi

                for gyro_data_point in gyro_data_points:
                    gyro_dx, gyro_dy, gyro_dz = gyro_data_point.get_deltas()

                    gyro_x, gyro_y, gyro_z = self.gyro.get_position()

                    # self.cx = (self.cx + gyro_x / self.gyro.freq) * self.combine_factor_gyro + accel_pitch * (1 - self.combine_factor_gyro)
                    # self.cy = (self.cy + gyro_y / self.gyro.freq) * self.combine_factor_gyro + accel_yav * (1 - self.combine_factor_gyro)
                    # self.cz = (self.cz + gyro_z / self.gyro.freq) * self.combine_factor_gyro + accel_roll * (1 - self.combine_factor_gyro)

                    self.cx = (self.cx + gyro_x / self.gyro.freq) * self.combine_factor_gyro + accel_yav * (1 - self.combine_factor_gyro)
                    self.cy = (self.cy + gyro_y / self.gyro.freq) * self.combine_factor_gyro + accel_pitch * (1 - self.combine_factor_gyro)
                    self.cz = (self.cz + gyro_z / self.gyro.freq) * self.combine_factor_gyro + accel_roll * (1 - self.combine_factor_gyro)

                    now = time.time()
                    delta_time = now - last_time
                    last_time = now

                    output = self.pid_inner.process(pid_time, 0.0, math.sin(self.cy * math.pi / 90.0) * 2)

                    # bump processing
                    if last_cy is None:
                        last_cy = self.cy
                    else:
                        angle_change = self.cy - last_cy
                        last_cy = self.cy

                        # Is it going in wrong direction - away from set point (zero)?
                        if (self.cy < 0 and angle_change < 0) or (self.cy > 0 and angle_change > 0):
                            if self.bump_found is None:
                                if abs(angle_change) >= self.bump_threshold:
                                    self.bump_found = now
                                    # print(f" --- got bump delaying {self.cy}, delta={angle_change}")

                            if self.bump_found is not None and self.bump_found + self.bump_delay <= now:
                                self.bump_found = None
                                self.bump_in_place = now

                            if self.bump_in_place is not None:
                                if self.bump_in_place + self.bump_len <= now:
                                    self.bump_in_place = None
                                    bump = 0
                                else:
                                    bump = self.bump_step + self.cy * self.bump_gain / 45.0
                                    # print(f" --- got bump acting {self.cy}, delta={bump}")
                                    if angle_change > 0:
                                        output -= bump
                                    else:
                                        output += bump

                        else:
                            bump = 0
                            self.bump_found = None
                            self.bump_in_place = None

                    # output = round(round(output * step_factor, 1) / step_factor, 2)
                    # output += math.sin(now * longitude) / amplitude

                    output -= math.sin(self.cy * math.pi / 180)

                    control = output

                    pid_time += 1 / self.freq

                    # sign = -1 if output < 0 else 1
                    # output = sign * output * output * self.expo + output * (1 - self.expo)
                    # if -self.output_dead_band < output < self.output_dead_band:
                    #     output = 0.0
                    # elif output > 0:
                    #     output += self.output_offset
                    # else:
                    #     output -= self.output_offset

                    self._logger.log(time.time(),
                                     gyro_dx, gyro_dy, gyro_dz,
                                     gyro_x, gyro_y, gyro_z,
                                     gyro_data_point.status, gyro_data_point.fifo_status, len(gyro_data_points),
                                     accel_dx, accel_dy, accel_dz,
                                     accel_x, accel_y, accel_z,
                                     accel_pitch, accel_roll, accel_yav,
                                     self.cx, self.cy, self.cz,
                                     self.pid_inner.p, self.pid_inner.i, self.pid_inner.d,
                                     self.pid_inner.p * self.pid_inner.kp, self.pid_inner.i * self.pid_inner.ki, self.pid_inner.d * self.pid_inner.kd,
                                     delta_time,
                                     output,
                                     control,
                                     bump)

                if self.state == self.STATE_WAITING_FOR_READY and -4 < self.cy < 4:
                    self.state = self.STATE_BALANCING
                    print("*** Got upright - starting balance!")
                elif self.state == self.STATE_BALANCING:
                    if self.cy < -self.max_deg or self.cy > self.max_deg:
                        self.state = self.STATE_WAITING_FOR_READY
                        self.motors.left_speed(0.0)
                        self.motors.right_speed(0.0)
                        print(f"*** Got over {self.max_deg} def stopping!")
                    else:
                        self.motors.left_speed(control)
                        self.motors.right_speed(control)

                if last_state != self.state:
                    last_state = self.state
                    if self.state != self.STATE_BALANCING:
                        print("*** State changed - stopped balancing!")
                        self.motors.left_speed(0.0)
                        self.motors.right_speed(0.0)

                had_exception = False
            except Exception as loop_exception:
                if not had_exception:
                    print("ERROR: " + str(loop_exception) + "\n" + ''.join(traceback.format_tb(loop_exception.__traceback__)))
                    had_exception = True

    def setup_logger(self):
        self.telemetry_server = telemetry.SocketTelemetryServer(port=self.telemetry_port)
        self._logger = self.telemetry_server.create_logger("balance-data")
        self._logger.add_word('gdx')
        self._logger.add_word('gdy')
        self._logger.add_word('gdz')
        self._logger.add_double('gx')
        self._logger.add_double('gy')
        self._logger.add_double('gz')
        self._logger.add_word('status')
        self._logger.add_byte('fifo_status')
        self._logger.add_byte('data_points')
        self._logger.add_word('adx')
        self._logger.add_word('ady')
        self._logger.add_word('adz')
        self._logger.add_double('ax')
        self._logger.add_double('ay')
        self._logger.add_double('az')
        self._logger.add_double('apitch')
        self._logger.add_double('aroll')
        self._logger.add_double('ayaw')
        self._logger.add_double('cx')
        self._logger.add_double('cy')
        self._logger.add_double('cz')
        self._logger.add_double('pi_p')
        self._logger.add_double('pi_i')
        self._logger.add_double('pi_d')
        self._logger.add_double('pi_pg')
        self._logger.add_double('pi_ig')
        self._logger.add_double('pi_dg')
        self._logger.add_double('pi_dt')
        self._logger.add_double('pi_o')
        self._logger.add_double('out')
        self._logger.add_double('bump')

        self._logger.init()
        self.telemetry_server.start()

    def calibrate(self, what_to_calibrate):
        if what_to_calibrate.startswith("gyro"):
            print("Calibrating gyro...")
            cx, cy, cz = self.gyro.calibrate(2)
            self.gyro.reset_position()
            print(f"Calibrating gyro finished. Values {cx}, {cy}, {cz}.")
        elif what_to_calibrate.startswith("accel"):
            print("Calibrating accel...")
            cx, cy, cz = self.accel.calibrate(2)
            print(f"Calibrating accel finished. Values {cx}, {cy}, {cz}.")
        elif what_to_calibrate.startswith("all"):
            print("Calibrating gyro...")
            cx, cy, cz = self.gyro.calibrate(2)
            self.gyro.reset_position()
            print(f"Calibrating gyro finished. Values {cx}, {cy}, {cz}.")
            print("Calibrating accel...")
            cx, cy, cz = self.accel.calibrate(2)
            print(f"Calibrating accel finished. Values {cx}, {cy}, {cz}.")
            self.cx = 0
            self.cy = 0
            self.cz = 0
        elif len(what_to_calibrate) == 0:
            print("Calibrating: need 'gyro' or 'accel' in message payload")

    def start(self):
        self.state = self.STATE_WAITING_FOR_READY
        print("  Started")

    def stop(self):
        self.state = self.STATE_STOPPED
        print("  Stopped")

    def setup_loop(self):
        print("    setting up loop thread...")
        self._thread = threading.Thread(target=self.run_loop, daemon=True)
        self._thread.start()
        print("    starting loop thread...")

    def init(self):
        self.setup_logger()
        self.telemetry_server.setup_accept_clients_thread()
        self.telemetry_server.setup_deferred()
        self.setup_loop()
        self.motors.init()
