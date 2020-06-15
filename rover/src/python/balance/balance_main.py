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


class BalancingService:
    def __init__(self):
        self._debug = False

        self.balance = Balance()

    def calibrate(self, _topic, payload, _groups):
        self.balance.calibrate(payload)

    def start_callback(self, _topic, _payload, _groups):
        self.balance.start()

    def stop_callback(self, _topic, _payload, _groups):
        self.balance.stop()

    def request_info(self, _topic, _payload, _groups):
        pyroslib.publish("balancing/info", f"telemetry_port={self.balance.telemetry_port}\n")

    def init(self):
        self.balance.init()


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

        print("Started balancing service.")

        pyroslib.forever(0.5, priority=pyroslib.PRIORITY_LOW)

    except Exception as ex:
        print("ERROR: " + str(ex) + "\n" + ''.join(traceback.format_tb(ex.__traceback__)))
