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
        'bump': {'threshold': 0.10, 'delay': 0.2, 'gain': 1.0, 'step': 0.2, 'len': 0.3},
        'gyro': {'filter': 0.3, 'bandwidth': 25},
        'accel': {'filter': 0.5},
        'combine_factor_gyro': 0.95,
        'freq': 100
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
            freq=int(balance_map['freq']),
            gyro_bandwidth=float(balance_map['gyro']['bandwidth']),
            gyro_filter=float(balance_map['gyro']['filter']),
            accel_filter=float(balance_map['accel']['filter']),
            pid_inner=PID().update_gains_from_map(balance_map['pid_inner']),
            pid_outer=PID().update_gains_from_map(balance_map['pid_outer']),
            combine_factor_gyro=float(balance_map['combine_factor_gyro']),
            bump_threshold=float(balance_map['bump']['threshold']),
            bump_delay=float(balance_map['bump']['delay']),
            bump_gain=float(balance_map['bump']['gain']),
            bump_step=float(balance_map['bump']['step']),
            bump_len=float(balance_map['bump']['len']),
        )
        self.balance.init()
        self.balance.state = Balance.STATE_WAITING_FOR_READY

    def gyro_filter_changed(self, _topic, payload, _groups):
        # print(f"Received gyro filter {_topic} : {payload}")
        self.balance.gyro.filter = float(payload)

    def accel_filter_changed(self, _topic, payload, _groups):
        # print(f"Received accel filter {_topic} : {payload}")
        self.balance.accel.filter = float(payload)

    def combine_factor_gyro_changed(self, _topic, payload, _groups):
        print(f"Received combine factor {_topic} : {payload}")
        self.balance.combine_factor_gyro = float(payload)

    def pid_inner_changed(self, topic, payload, _groups):
        # print(f"Received inner {topic} : {payload}")
        topic = topic[32:]
        # noinspection PyBroadException
        try:
            if "p" == topic:
                self.balance.pid_inner.kp = float(payload)
                # print(f"  ... updated pi_p to {self.balance.pid_inner.kp}")
            if "i" == topic:
                self.balance.pid_inner.ki = float(payload)
                # print(f"  ... updated pi_i to {self.balance.pid_inner.ki}")
            if "d" == topic:
                self.balance.pid_inner.kd = float(payload)
                # print(f"  ... updated pi_d to {self.balance.pid_inner.kd}")
            if "g" == topic:
                self.balance.pid_inner.kg = float(payload)
                # print(f"  ... updated pi_g to {self.balance.pid_inner.kg}")
        except Exception as e:
            print("ERROR: " + str(e) + "\n" + ''.join(traceback.format_tb(e.__traceback__)))

    def pid_outer_changed(self, topic, payload, _groups):
        # print(f"Received outer {topic} : {payload}")
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
        except Exception as e:
            print("ERROR: " + str(e) + "\n" + ''.join(traceback.format_tb(e.__traceback__)))

    def bump_changed(self, topic, payload, _groups):
        print(f"Received outer {topic} : {payload}")
        topic = topic[27:]
        # noinspection PyBroadException
        try:
            if "threshold" == topic:
                self.balance.bump_threshold = float(payload)
                print(f"Got threshold of {self.balance.bump_threshold}")
            if "delay" == topic:
                self.balance.bump_delay = float(payload)
            if "gain" == topic:
                self.balance.bump_gain = float(payload)
            if "step" == topic:
                self.balance.bump_step = float(payload)
            if "len" == topic:
                self.balance.bump_len = float(payload)
        except Exception as e:
            print("ERROR: " + str(e) + "\n" + ''.join(traceback.format_tb(e.__traceback__)))


if __name__ == "__main__":
    try:
        import pyroslib

        print("Starting balancing service...")

        pyroslib.init("balancing-service")

        balancing_service = BalancingService()
        balancing_service.init()
        print("    finished setting up main loop")

        pyroslib.subscribe("storage/write/balance/gyro/filter", balancing_service.gyro_filter_changed)
        pyroslib.subscribe("storage/write/balance/accel/filter", balancing_service.accel_filter_changed)
        pyroslib.subscribe("storage/write/balance/combine_factor_gyro", balancing_service.combine_factor_gyro_changed)
        # pyroslib.subscribe("storage/write/balance/pid_inner/#", balancing_service.pid_inner_changed)
        pyroslib.subscribe("storage/write/balance/pid_inner/p", balancing_service.pid_inner_changed)
        pyroslib.subscribe("storage/write/balance/pid_inner/i", balancing_service.pid_inner_changed)
        pyroslib.subscribe("storage/write/balance/pid_inner/d", balancing_service.pid_inner_changed)
        pyroslib.subscribe("storage/write/balance/pid_inner/g", balancing_service.pid_inner_changed)
        # pyroslib.subscribe("storage/write/balance/pid_outer/#", balancing_service.pid_outer_changed)
        pyroslib.subscribe("storage/write/balance/pid_outer/p", balancing_service.pid_outer_changed)
        pyroslib.subscribe("storage/write/balance/pid_outer/i", balancing_service.pid_outer_changed)
        pyroslib.subscribe("storage/write/balance/pid_outer/d", balancing_service.pid_outer_changed)
        pyroslib.subscribe("storage/write/balance/pid_outer/g", balancing_service.pid_outer_changed)
        # pyroslib.subscribe("storage/write/balance/bump/#", balancing_service.bump_changed)
        pyroslib.subscribe("storage/write/balance/bump/threshold", balancing_service.bump_changed)
        pyroslib.subscribe("storage/write/balance/bump/delay", balancing_service.bump_changed)
        pyroslib.subscribe("storage/write/balance/bump/step", balancing_service.bump_changed)
        pyroslib.subscribe("storage/write/balance/bump/gain", balancing_service.bump_changed)

        pyroslib.subscribe("balancing/calibrate", balancing_service.calibrate)
        pyroslib.subscribe("balancing/start", balancing_service.start_callback)
        pyroslib.subscribe("balancing/stop", balancing_service.stop_callback)
        pyroslib.subscribe("balancing/request-info", balancing_service.request_info)

        print("Started balancing service.")

        pyroslib.forever(0.5, priority=pyroslib.PRIORITY_LOW)

    except Exception as ex:
        print("ERROR: " + str(ex) + "\n" + ''.join(traceback.format_tb(ex.__traceback__)))
