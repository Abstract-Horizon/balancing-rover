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

import pygame
import time

from pyros_support_ui.components import Component, ALIGNMENT


class GraphData:
    def __init__(self, maximum, minimum):
        self.max = maximum
        self.min = minimum
        self.name = ""

    def fetch(self, from_timestamp, to_timestamp):
        return []


class AutoScalingData(GraphData):
    def __init__(self, maximum, minimum):
        super(AutoScalingData, self).__init__(maximum, minimum)
        self._default_min = minimum
        self._default_max = maximum
        self._last_min_seen = None
        self._last_max_seen = None

    def auto_scale_data(self, data):
        max_seen = False
        max_found = -100000000
        min_seen = False
        min_found = 100000000
        for i in range(len(data)):
            if self.max < data[i][1]:
                self.max = data[i][1]
                self._last_max_seen = data[i][0]
                max_seen = True
            elif not max_seen and max_found < data[i][1]:
                max_found = data[i][1]
            if self.min > data[i][1]:
                self.min = data[i][1]
                self._last_min_seen = data[i][0]
                min_seen = True
            elif not min_seen and min_found > data[i][1]:
                min_found = data[i][1]
        if not max_seen:
            if max_found < self._default_max:
                max_found = self._default_max
            if max_found < self.max:
                self.max = max_found
        if not min_seen:
            if min_found > self._default_min:
                min_found = self._default_min
            if min_found > self.min:
                self.min = min_found


class TelemetryGraphData(AutoScalingData):
    def __init__(self, telemetry_client, stream, column_name, maximum, minimum, auto_scale=False):
        super(TelemetryGraphData, self).__init__(maximum, minimum)
        self.telemetry_client = telemetry_client
        self.stream = stream
        self.name = column_name
        self.column_index = -1
        self.collect = True
        self.auto_scale = auto_scale

        for i, field in enumerate(self.stream.get_fields()):
            if field.name == column_name:
                self.column_index = i

    def fetch(self, from_timestamp, to_timestamp):
        result = []

        if self.collect:
            def fetch(data):
                result.extend([[d[0], d[self.column_index + 1]] for d in data])
                if self.auto_scale:
                    self.auto_scale_data(result)

            self.telemetry_client.retrieve(self.stream, from_timestamp, to_timestamp, fetch)
        return result


class ChangedSingleTelemetryGraphData(TelemetryGraphData):
    def __init__(self, telemetry_client, stream, column_name, maximum, minimum, transform, auto_scale=False):
        super(ChangedSingleTelemetryGraphData, self).__init__(telemetry_client, stream, column_name, maximum, minimum, auto_scale=auto_scale)
        self.transform = transform

    def fetch(self, from_timestamp, to_timestamp):
        result = []

        if self.collect:
            def fetch(data):
                result.extend([[d[0], d[self.column_index + 1]] for d in data])
                for i in range(len(result)):
                    result[i][1] = self.transform(result[i][1])
                if self.auto_scale:
                    self.auto_scale_data(result)

            self.telemetry_client.retrieve(self.stream, from_timestamp, to_timestamp, fetch)
        return result


class GraphController:
    def __init__(self, timepoint=-1, timewindow=10):
        self.timepoint = timepoint
        self.timewindow = timewindow

    def pause(self):
        self.timepoint = time.time() - self.timewindow

    def resume(self):
        self.timepoint = -1

    def get_timepoint(self):
        return self.timepoint

    def get_timewindow(self):
        return self.timewindow


class Graph(Component):
    # noinspection PyArgumentList
    def __init__(self, rect, ui_factory, small_font=None, controller=None):
        super(Graph, self).__init__(rect)
        self.graph_data = None
        self.border_colour = ui_factory.colour
        self.inner_colour = pygame.color.THECOLORS['white']
        self.background_colour = pygame.color.THECOLORS['black']
        self.label_colour = pygame.color.THECOLORS['grey']
        self.line_colour = (96, 96, 96)
        self.inner_rect = None
        self.units = ''
        # self.graph_time_len=graph_time_len
        self.controller = controller if controller is not None else self

        # self.min_width_time = 60

        self.small_font = small_font if small_font is not None else ui_factory.get_small_font()

        self.title = ui_factory.label(None, "", colour=self.label_colour, h_alignment=ALIGNMENT.CENTER, v_alignment=ALIGNMENT.TOP)
        self.max_label = ui_factory.label(None, "", colour=self.label_colour, h_alignment=ALIGNMENT.LEFT, v_alignment=ALIGNMENT.TOP)
        self.min_label = ui_factory.label(None, "", colour=self.label_colour, h_alignment=ALIGNMENT.LEFT, v_alignment=ALIGNMENT.BOTTOM)
        self.now_label = ui_factory.label(None, "now", colour=self.label_colour, h_alignment=ALIGNMENT.RIGHT, v_alignment=ALIGNMENT.BOTTOM)
        self.time_label = ui_factory.label(None, "", colour=self.label_colour, h_alignment=ALIGNMENT.CENTER, v_alignment=ALIGNMENT.BOTTOM)
        # self.warning_value = -1
        # self.critical_value = -1
        # self.warning_colour = pygame.color.THECOLORS['orange']
        # self.critical_colour = pygame.color.THECOLORS['red']

        # self.redefine_rect(rect)
        self.timepoint = -1
        self.timewindow = 10
        self._min_value = 0
        self._max_value = 0

    def get_timepoint(self):
        return self.timepoint

    def get_timewindow(self):
        return self.timewindow

    def redefine_rect(self, rect):
        super(Graph, self).redefine_rect(rect)
        self.inner_rect = rect.inflate(-3, -2)
        self.title.redefine_rect(self.inner_rect)
        self.max_label.redefine_rect(self.inner_rect)
        self.min_label.redefine_rect(self.inner_rect)
        self.now_label.redefine_rect(self.inner_rect)
        self.time_label.redefine_rect(self.inner_rect)

    def set_graph_data(self, graph_data):
        self.graph_data = graph_data
        self.title.set_text(graph_data.name)
        self._max_value = graph_data.max
        self._min_value = graph_data.min
        self._set_max_label(graph_data.max)
        self._set_min_label(graph_data.min)

    def _set_max_label(self, _max):
        self.max_label.set_text(f"{round(_max, 3)}")

    def _set_min_label(self, _min):
        self.min_label.set_text(f"{round(_min, 3)}")

    def draw(self, surface):
        pygame.draw.rect(surface, self.border_colour, self.rect, 1)
        pygame.draw.rect(surface, self.background_colour, self.inner_rect)
        if self.graph_data is not None:

            timepoint = self.controller.get_timepoint()
            timewindow = self.controller.get_timewindow()

            timepoint = timepoint if timepoint >= 0 else time.time() - timewindow

            data = self.graph_data.fetch(timepoint, timepoint + timewindow + 0.01)
            graph_last_timepoint = timepoint + timewindow

            if self._max_value != self.graph_data.max:
                self._set_max_label(self.graph_data.max)
                self._max_value = self.graph_data.max
            if self._min_value != self.graph_data.min:
                self._set_min_label(self.graph_data.min)
                self._min_value = self.graph_data.min

            if len(data) > 0:
                t0 = data[0][0]
                now = time.time()

                t_minutes = now - t0
                if t_minutes < 0.1:
                    self.time_label.set_text("")
                elif int(t_minutes) < 60:
                    self.time_label.set_text(str(int(t_minutes)) + " s")
                elif int(t_minutes) == 60:
                    self.time_label.set_text("1 min")
                else:
                    self.time_label.set_text(str(int(t_minutes / 60)) + " mins")

                data_time_width = now - t0
                # if data_time_width < self.min_width_time:
                #     data_time_width = self.min_width_time
                # t_max = t0 + data_width
                # d_max = self.max_value

                if data_time_width <= 20:
                    minute_line_time = 1
                elif data_time_width <= 60:
                    minute_line_time = 5
                elif data_time_width <= 300:
                    minute_line_time = 25
                else:
                    minute_line_time = 300

                if data_time_width > timewindow:
                    data_time_width = timewindow

                tlast = data[-1][0]
                while t0 < tlast - data_time_width:
                    del data[0]
                    t0 = data[0][0]

                t = t0 + minute_line_time
                while t < tlast:
                    x = self.inner_rect.right - (graph_last_timepoint - t) * self.inner_rect.width / timewindow
                    pygame.draw.line(surface, self.line_colour, (x, self.inner_rect.y + 1), (x, self.inner_rect.bottom - 2), 1)
                    t += minute_line_time

                data_range = self.graph_data.max - self.graph_data.min

                if self.graph_data.min <= 0:
                    y = int(self.inner_rect.bottom + self.graph_data.min * self.inner_rect.height / data_range)
                    pygame.draw.line(surface, self.line_colour, (self.inner_rect.x, y), (self.inner_rect.right, y), 1)

                points = []
                for d in data:
                    t = d[0]
                    p = d[1]
                    # print(f"{t} : {p}")
                    if p > self.graph_data.max:
                        p = self.graph_data.max
                    elif p < self.graph_data.min:
                        p = self.graph_data.min

                    p -= self.graph_data.min

                    x = int(self.inner_rect.right - (graph_last_timepoint - t) * self.inner_rect.width / timewindow)
                    y = int(self.inner_rect.bottom - p * self.inner_rect.height / data_range)
                    points.append((x, y))

                if len(points) > 1:
                    pygame.draw.lines(surface, self.inner_colour, False, points)

                # points.append((x, self.inner_rect.bottom))
                # points.append((self.inner_rect.x, self.inner_rect.bottom))
                # pygame.draw.polygon(surface, self.inner_colour, points)
                # pygame.draw.polygon(surface, self.border_colour, points, 1)

                # if self.warning_value >= 0:
                #     y = self.inner_rect.bottom - self.warning_value * self.inner_rect.height / self.max_value
                #     pygame.draw.line(surface, self.warning_colour, (self.inner_rect.x + 1, y), (self.inner_rect.right - 2, y))
                #
                # if self.critical_value >= 0:
                #     y = self.inner_rect.bottom - self.critical_value * self.inner_rect.height / self.max_value
                #     pygame.draw.line(surface, self.critical_colour, (self.inner_rect.x + 1, y), (self.inner_rect.right - 2, y))

            self.title.draw(surface)
            self.max_label.draw(surface)
            self.min_label.draw(surface)
            self.now_label.draw(surface)
            self.time_label.draw(surface)
