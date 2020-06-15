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


class Balance:
    def __init__(self):
        self._debug = False

        self.gyro = L3G4200D(freq=200)
        self.accel = ADXL345()
        self.combine_factor_gyro = 0.95
        self.cx = 0.0
        self.cy = 0.0
        self.cz = 0.0

        self.telemetry_port = 1860

        self._thread = None
        self._logger = None
        self.telemetry_server = None
        self._is_running = False

    def run_loop(self):
        print("    started loop thread.")
        had_exception = False
        last_log = 0

        while True:
            try:
                if self._debug and time.time() - last_log > 5:
                    last_log = time.time()
                    do_log_now = True
                    print("Log loop:")
                else:
                    do_log_now = False

                if do_log_now:
                    print("  Preparing to read gyro data")
                gyro_data_points = self.gyro.read_deltas()
                if do_log_now:
                    print("  Prepareing to read accel data")
                accel_data_point = self.accel.read()

                if do_log_now:
                    print(f"  Got gyro data ({len(gyro_data_points)} data points)...")

                if self._is_running:
                    accel_dx, accel_dy, accel_dz = accel_data_point.get_raw_data()
                    accel_x, accel_y, accel_z = accel_data_point.get_data()

                    accel_pitch = (math.atan2(accel_z, math.sqrt(accel_x * accel_x + accel_y * accel_y)) * 180.0) / math.pi
                    accel_roll = (math.atan2(accel_x, (math.sqrt(accel_z * accel_z + accel_y * accel_y))) * 180.0) / math.pi
                    accel_yav = (math.atan2(accel_y, (math.sqrt(accel_z * accel_z + accel_x * accel_x))) * 180.0) / math.pi

                    for gyro_data_point in gyro_data_points:
                        if do_log_now:
                            print(f"    Preparing to get gyro delta data.")

                        gyro_dx, gyro_dy, gyro_dz = gyro_data_point.get_deltas()

                        if do_log_now:
                            print(f"    Preparing to get gyro position data.")

                        gyro_x, gyro_y, gyro_z = self.gyro.get_position()

                        self.cx = (self.cx + gyro_x / self.gyro.freq) * self.combine_factor_gyro + accel_pitch * (1 - self.combine_factor_gyro)
                        self.cy = (self.cy + gyro_y / self.gyro.freq) * self.combine_factor_gyro + accel_yav * (1 - self.combine_factor_gyro)
                        self.cz = (self.cz + gyro_z / self.gyro.freq) * self.combine_factor_gyro + accel_roll * (1 - self.combine_factor_gyro)

                        if do_log_now:
                            print(f"    Preparing to log a data point... Have {len(self.telemetry_server._client_sockets)} clients.")
                        self._logger.log(time.time(),
                                         gyro_dx, gyro_dy, gyro_dz,
                                         gyro_x, gyro_y, gyro_z,
                                         gyro_data_point.status, gyro_data_point.fifo_status, len(gyro_data_points),
                                         accel_dx, accel_dy, accel_dz,
                                         accel_x, accel_y, accel_z,
                                         accel_pitch, accel_roll, accel_yav,
                                         self.cx, self.cy, self.cz)
                        if do_log_now:
                            print("Logged.")

                had_exception = False
            except Exception as loop_exception:
                if not had_exception:
                    print("ERROR: " + str(loop_exception) + "\n" + ''.join(traceback.format_tb(loop_exception.__traceback__)))
                    had_exception = True

    def setup_logger(self):
        self.telemetry_server = telemetry.SocketTelemetryServer(port=self.telemetry_port)
        self._logger = self.telemetry_server.create_logger("balance-data")
        self._logger.add_int('gdx')
        self._logger.add_int('gdy')
        self._logger.add_int('gdz')
        self._logger.add_double('gx')
        self._logger.add_double('gy')
        self._logger.add_double('gz')
        self._logger.add_word('status')
        self._logger.add_byte('fifo_status')
        self._logger.add_byte('data_points')
        self._logger.add_int('adx')
        self._logger.add_int('ady')
        self._logger.add_int('adz')
        self._logger.add_double('ax')
        self._logger.add_double('ay')
        self._logger.add_double('az')
        self._logger.add_double('apitch')
        self._logger.add_double('aroll')
        self._logger.add_double('ayaw')
        self._logger.add_double('cx')
        self._logger.add_double('cy')
        self._logger.add_double('cz')

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
        self._is_running = True

        # self.gyro.reset_position()
        self.gyro.start()

        print("  Started")

    def stop(self):
        self._is_running = False

        self.gyro.idle()

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
