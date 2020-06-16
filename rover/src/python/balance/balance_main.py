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

from balance.balancing import Balance
from balance.pid import PID
from storage import storagelib


class BalancingService:
    _TEMPLATE_STORAGE = {
        'pid_inner': {'p': 0.75, 'i': 0.2, 'd': 0.05, 'g': 1.0},
        'pid_outer': {'p': 0.75, 'i': 0.2, 'd': 0.05, 'g': 1.0},
        'gyro': {'filter': 0.3, 'freq': 200, 'bandwidth': 50},
        'accel': {'filter': 0.5, 'freq': 200},
        'combine_factor_gyro': 0.95
    }

    def __init__(self):
        self._debug = False
        self.balance = None

    def calibrate(self, _topic, payload, _groups):
        self.balance.calibrate(payload)

    def start_callback(self, _topic, _payload, _groups):
        self.balance.start()

    def stop_callback(self, _topic, _payload, _groups):
        self.balance.stop()

    def request_info(self, _topic, _payload, _groups):
        pyroslib.publish("balancing/info", f"telemetry_port={self.balance.telemetry_port}\n")

    def init(self):

        storagelib.subscribe_with_prototype("balance", self._TEMPLATE_STORAGE)
        storagelib.wait_for_data()
        storagelib.bulk_populate_if_empty("balance", self._TEMPLATE_STORAGE)

        balance_map = storagelib.storage_map["balance"]

        print(f"  ... balance_map:  {balance_map}")

        self.balance = Balance(
            gyro_freq=int(balance_map['gyro']['freq']),
            gyro_bandwidth=float(balance_map['gyro']['bandwidth']),
            gyro_filter=float(balance_map['gyro']['filter']),
            accel_freq=int(balance_map['accel']['freq']),
            accel_filter=float(balance_map['accel']['filter']),
            pid_inner=PID().update_gains_from_map(balance_map['pid_inner']),
            pid_outer=PID().update_gains_from_map(balance_map['pid_outer']),
            combine_factor_gyro=float(balance_map['combine_factor_gyro'])
        )
        self.balance.init()

    def gyro_filter_changed(self, _topic, payload, _groups):
        self.balance.gyro.filter = float(payload)

    def accel_filter_changed(self, _topic, payload, _groups):
        self.balance.accel.filter = float(payload)

    def combine_factor_gyro_changed(self, _topic, payload, _groups):
        self.balance.combine_factor_gyro = float(payload)

    def pid_inner_changed(self, topic, payload, _groups):
        topic = topic[32:]
        # noinspection PyBroadException
        try:
            if "p" == topic:
                self.balance.pid_inner.kp = float(payload)
            if "i" == topic:
                self.balance.pid_inner.ki = float(payload)
            if "d" == topic:
                self.balance.pid_inner.kd = float(payload)
            if "g" == topic:
                self.balance.pid_inner.kg = float(payload)
        except Exception:
            pass

    def pid_outer_changed(self, topic, payload, _groups):
        topic = topic[32:]
        # noinspection PyBroadException
        try:
            if "p" == topic:
                self.balance.pid_outer.kp = float(payload)
            if "i" == topic:
                self.balance.pid_outer.ki = float(payload)
            if "d" == topic:
                self.balance.pid_outer.kd = float(payload)
            if "g" == topic:
                self.balance.pid_outer.kg = float(payload)
        except Exception:
            pass


if __name__ == "__main__":
    try:
        import pyroslib

        print("Starting balancing service...")

        pyroslib.init("balancing-service")

        balancing_service = BalancingService()
        balancing_service.init()
        print("    finished setting up main loop")

        pyroslib.subscribe("balancing/calibrate", balancing_service.calibrate)
        pyroslib.subscribe("balancing/start", balancing_service.start_callback)
        pyroslib.subscribe("balancing/stop", balancing_service.stop_callback)
        pyroslib.subscribe("balancing/request-info", balancing_service.request_info)

        pyroslib.subscribe("storage/write/balance/gyro/filter", balancing_service.gyro_filter_changed)
        pyroslib.subscribe("storage/write/balance/accel/filter", balancing_service.accel_filter_changed)
        pyroslib.subscribe("storage/write/balance/combine_factor_gyro", balancing_service.combine_factor_gyro_changed)
        pyroslib.subscribe("storage/write/balance/pid_inner/#", balancing_service.pid_inner_changed)
        pyroslib.subscribe("storage/write/balance/pid_outer/#", balancing_service.pid_inner_changed)

        print("Started balancing service.")

        pyroslib.forever(0.5, priority=pyroslib.PRIORITY_LOW)

    except Exception as ex:
        print("ERROR: " + str(ex) + "\n" + ''.join(traceback.format_tb(ex.__traceback__)))
