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

from __future__ import annotations

import time


class PID:
    def __init__(self, p_gain=0.75, i_gain=0.2, d_gain=0.05, gain=1.0, dead_band=0.0001, i_gain_scale=1.0, d_gain_scale=100.0, difference=lambda x, y: x - y):
        self.set_point = 0.0
        self.p = 0.0
        self.i = 0.0
        self.d = 0.0
        self.kp = p_gain
        self.ki = i_gain
        self.kd = d_gain
        self.kg = gain
        self.i_gain_scale = i_gain_scale
        self.d_gain_scale = d_gain_scale
        self.dead_band = dead_band
        self.last_error = 0.0
        self.last_time = 0.0
        self.last_output = 0.0
        self.last_delta = 0.0
        self.first = True
        self.difference = difference

    def update_gains_from_map(self, gains_map) -> PID:
        def to_value(name, default):
            # noinspection PyBroadException
            try:
                return float(gains_map[name])
            except Exception:
                return default

        self.kp = to_value('p', 0.75)
        self.ki = to_value('i', 0.2)
        self.kd = to_value('d', 0.05)
        self.kg = to_value('g', 1.0)
        return self

    def set_difference_method(self, difference):
        self.difference = difference

    @staticmethod
    def angle_difference(a1, a2):
        diff = a1 - a2
        if diff >= 180:
            return diff - 360
        elif diff <= -180:
            return diff + 360
        return diff

    def process(self, set_point, current):
        now = time.time()

        error = self.difference(set_point, current)
        if abs(error) <= self.dead_band:
            error = 0.0

        if self.first:
            self.first = False
            self.set_point = set_point
            self.last_error = error
            self.last_time = now
            return 0
        else:
            delta_time = now - self.last_time

            self.p = error
            if self.last_error < 0 < error or self.last_error > 0 > error:
                self.i = 0.0
            elif abs(error) <= 0.1:
                self.i = 0.0
            else:
                self.i += error * delta_time * self.i_gain_scale

            if delta_time > 0:
                self.d = (error - self.last_error) / (delta_time * self.d_gain_scale)

            output = self.p * self.kp + self.i * self.ki + self.d * self.kd

            output *= self.kg

            self.set_point = set_point
            self.last_output = output
            self.last_error = error
            self.last_time = now
            self.last_delta = delta_time

        return output

    def __repr__(self):
        return f"PID(kp={self.kp}, ki={self.ki}, kd={self.kd}, kg={self.kg}, db={self.dead_band}; p={self.p * self.kp}, i={self.i * self.ki}, d={self.d * self.kd}, dt={self.last_delta})"
