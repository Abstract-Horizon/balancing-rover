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

import sys
import pygame
import pyros
import threading
import traceback
import time

from functools import partial

from pygame import Rect

from pyros_support_ui import PyrosClientApp
from pyros_support_ui.components import UIAdapter, Component, Collection, LeftRightLayout, UiHint, TopDownLayout, CardsCollection
from pyros_support_ui.pygamehelper import load_font, load_image

from pyros_support_ui.box_blue_sf_theme import BoxBlueSFThemeFactory
# from pyros_support_ui.flat_theme import FlatThemeFactory

from graph_component import Graph, GraphController, TelemetryGraphData, ChangedSingleTelemetryGraphData
from number_input import NumberInputComponent

from telemetry import CachingSocketTelemetryClient


ui_adapter = UIAdapter(screen_size=(1400, 848))
ui_factory = BoxBlueSFThemeFactory(ui_adapter, font=load_font("garuda.ttf", 20), small_font=load_font("garuda.ttf", 14))

pyros_client_app = PyrosClientApp(ui_factory,
                                  logo_image=load_image("GCC_coloured_small.png"),
                                  logo_alt_image=load_image("GCC_green_small.png"),
                                  connect_to_first=True,
                                  connect_to_only=False)

pyros_client_app.pyros_init("balancing-rover-#")
ui_adapter.set_top_component(pyros_client_app)


class CommandsPanel(Collection):
    def __init__(self):
        super(CommandsPanel, self).__init__(None, layout=LeftRightLayout(margin=10))


class GraphsPanel(Collection):
    def __init__(self, rows, columns, graph_controller=None):
        super(GraphsPanel, self).__init__(None)
        self.rows = rows
        self.columns = columns
        self.main_graph = Graph(None, ui_factory, controller=graph_controller)
        self.add_component(self.main_graph)
        self.graphs = []
        for i in range(rows):
            self.graphs.append([])
            for _ in range(columns):
                graph = Graph(None, ui_factory, controller=graph_controller)
                self.graphs[i].append(graph)
                self.add_component(graph)

    def redefine_rect(self, rect):
        self.rect = rect

        margin = 4

        if self.rows > 2:
            self.main_graph.redefine_rect(Rect(rect.x, rect.y, rect.width, rect.height // 2 - margin))
        else:
            self.main_graph.redefine_rect(Rect(rect.x, rect.y, rect.width, rect.height * 2 // 3 - margin))

        x = rect.x
        y = self.main_graph.rect.bottom + margin
        width = (rect.width - margin * (self.columns - 1)) // self.columns
        height = (rect.bottom - y - margin * (self.rows - 1)) // self.rows
        for r in range(self.rows):
            for c in range(self.columns):
                self.graphs[r][c].redefine_rect(Rect(x + (width + margin) * c, y, width, height))

            y = y + height + margin


class ValuesPanel(Collection):
    def __init__(self, rect, _ui_factory):
        super(ValuesPanel, self).__init__(rect, TopDownLayout(margin=5))
        self.cached_values = {
            'pid_inner': {'p': 0.75, 'i': 0.2, 'd': 0.05, 'g': 1.0},
            'pid_outer': {'p': 0.75, 'i': 0.2, 'd': 0.05, 'g': 1.0},
            'gyro': {'filter': 0.3, 'freq': 200, 'bandwidth': 50},
            'accel': {'filter': 0.5, 'freq': 200},
            'combine_factor_gyro': 0.95,
            'expo': 0.2
        }
        self.manual = 0.01

        pyros.subscribe("storage/write/balance/gyro/filter", self.gyro_filter_changed)
        pyros.subscribe("storage/write/balance/accel/filter", self.accel_filter_changed)
        pyros.subscribe("storage/write/balance/combine_factor_gyro", self.combine_factor_gyro_changed)
        pyros.subscribe("storage/write/balance/expo", self.expo_changed)
        pyros.subscribe("storage/write/balance/pid_inner/#", self.pid_inner_changed)
        pyros.subscribe("storage/write/balance/pid_outer/#", self.pid_inner_changed)

        button_height = 40

        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("gyro/filter"), 'g-f', button_font=_ui_factory.get_small_font()))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("accel/filter"), 'a-f', button_font=_ui_factory.get_small_font()))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("combine_factor_gyro"), 'c-f', button_font=_ui_factory.get_small_font()))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("expo"), 'expo', button_font=_ui_factory.get_small_font()))
        self.add_component(Component(Rect(0, 0, 300, 10)))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("pid_inner/p"), 'pi-p', button_font=_ui_factory.get_small_font()))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("pid_inner/i"), 'pi-i', button_font=_ui_factory.get_small_font()))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("pid_inner/d"), 'pi-d', button_font=_ui_factory.get_small_font()))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, self.create_getter_and_setter("pid_inner/g"), 'pi-g', button_font=_ui_factory.get_small_font()))
        self.add_component(Component(Rect(0, 0, 300, 10)))
        self.add_component(NumberInputComponent(Rect(0, 0, 300, button_height), _ui_factory, (self.get_manual, self.set_manual), 'man', button_font=_ui_factory.get_small_font(), top_scale=2, bottom_scale = 0))

        pyros_client_app.add_on_connected_subscriber(self.read_values)

    def read_values(self):
        def send_read_recursively(_map, path):
            for key in _map:
                if type(_map[key]) is dict:
                    send_read_recursively(_map[key], (path + "/" + key) if path != "" else key)
                else:
                    subscribe_topic = f"storage/read/balance/{path}/{key}"
                    print(f"Subscribing to {subscribe_topic}")
                    pyros.publish(subscribe_topic, "")

        send_read_recursively(self.cached_values, "")

    def redefine_rect(self, rect):
        super(ValuesPanel, self).redefine_rect(rect)

    def gyro_filter_changed(self, _topic, payload, _groups):
        print(f"Gyro filter {_topic} : {payload}")
        self.cached_values['gyro']['filter'] = float(payload)

    def accel_filter_changed(self, _topic, payload, _groups):
        print(f"Accel filter {_topic} : {payload}")
        self.cached_values['accel']['filter'] = float(payload)

    def combine_factor_gyro_changed(self, _topic, payload, _groups):
        print(f"Combine factor {_topic} : {payload}")
        self.cached_values['combine_factor_gyro'] = float(payload)

    def expo_changed(self, _topic, payload, _groups):
        print(f"Expo {_topic} : {payload}")
        self.cached_values['expo'] = float(payload)

    def pid_inner_changed(self, topic, payload, _groups):
        print(f"Received inner {topic} : {payload}")
        topic = topic[32:]
        # noinspection PyBroadException
        try:
            if "p" == topic:
                self.cached_values['pid_inner']['p'] = float(payload)
            if "i" == topic:
                self.cached_values['pid_inner']['i'] = float(payload)
            if "d" == topic:
                self.cached_values['pid_inner']['d'] = float(payload)
            if "g" == topic:
                self.cached_values['pid_inner']['g'] = float(payload)
        except Exception:
            pass

    def pid_outer_changed(self, topic, payload, _groups):
        print(f"Received outer {topic} : {payload}")
        topic = topic[32:]
        # noinspection PyBroadException
        try:
            if "p" == topic:
                self.cached_values['pid_inner']['p'] = float(payload)
            if "i" == topic:
                self.cached_values['pid_inner']['i'] = float(payload)
            if "d" == topic:
                self.cached_values['pid_inner']['d'] = float(payload)
            if "g" == topic:
                self.cached_values['pid_inner']['g'] = float(payload)
        except Exception:
            pass

    def create_getter_and_setter(self, path):
        def read_value(map_place, name):
            return map_place[name]

        def write_value(map_place, name, _path, value):
            map_place[name] = value

            if abs(value > 0.1):
                s = "{0:.2f}".format(value)
            elif abs(value > 0.01):
                s = "{0:.3f}".format(value)
            elif abs(value > 0.001):
                s = "{0:.4f}".format(value)
            elif abs(value > 0.0001):
                s = "{0:.5f}".format(value)
            else:
                s = "{0:.6f}".format(value)

            pyros.publish("storage/write/balance/" + _path, s)

        splt = path.split('/')
        m = self.cached_values
        for p in splt[:-1]:
            m = m[p]
        return partial(read_value, m, splt[-1]), partial(write_value, m, splt[-1], path)

    def set_manual(self, manual):
        self.manual = manual

    def get_manual(self):
        return self.manual


class BalancingRoverTelemetry(Collection):
    def __init__(self, _ui_factory, _pyros_client_app):
        super(BalancingRoverTelemetry, self).__init__(None)
        self.pyros_client_app = _pyros_client_app
        self.commands = CommandsPanel()
        self.add_component(self.commands)
        self.graphs_panel = CardsCollection(Rect(0, 0, 100, 100))
        self.add_component(self.graphs_panel)

        self.graph_controller = GraphController()

        self.sensors_graphs_panel = GraphsPanel(3, 4, graph_controller=self.graph_controller)
        self.graphs_panel.add_card("sensors", self.sensors_graphs_panel)

        self.pid_graphs_panel = GraphsPanel(2, 3, graph_controller=self.graph_controller)
        self.graphs_panel.add_card("pid", self.pid_graphs_panel)

        self.graphs_panel.select_card("pid")

        self.values_panel = ValuesPanel(Rect(0, 0, 300, 800), _ui_factory)
        self.add_component(self.values_panel)

        self.start_stop_balancing_panel = Collection(Rect(0, 0, 100, 0))
        self.commands.add_component(self.start_stop_balancing_panel)
        self.start_stop_balancing_panel.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Start", on_click=self.start_balancing))
        self.start_stop_balancing_panel.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Stop", on_click=self.stop_balancing, hint=UiHint.WARNING))
        self.start_stop_balancing_panel.components[0].set_visible(False)
        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Manual", on_click=self.manual))

        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Calibrate", on_click=self.calibrate))

        self.start_stop_collecting_panel = Collection(Rect(0, 0, 200, 0))
        self.commands.add_component(self.start_stop_collecting_panel)
        self.start_stop_collecting_panel.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Stop Collecting", on_click=self.stop_collecting))
        self.start_stop_collecting_panel.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Continue Collecting", on_click=self.continue_collecting))
        self.start_stop_collecting_panel.components[0].set_visible(False)

        self.sensors_pid_switch_panel = Collection(Rect(0, 0, 200, 0))
        self.commands.add_component(self.sensors_pid_switch_panel)
        self.sensors_pid_switch_panel.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Show PID", on_click=self.show_pid))
        self.sensors_pid_switch_panel.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Show Sensors", on_click=self.show_sensors))
        self.sensors_pid_switch_panel.components[0].set_visible(False)

        self.pause_graph_panel = Collection(Rect(0, 0, 200, 0))
        self.commands.add_component(self.pause_graph_panel)
        self.pause_graph_panel.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Resume Graph", on_click=self.resume_graph))
        self.pause_graph_panel.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Pause Graph", on_click=self.pause_graph))
        self.pause_graph_panel.components[0].set_visible(False)

        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Clear", on_click=self.clear_graph))
        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Save All", on_click=self.save_graph_all))
        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Save ", on_click=self.save_graph))

        self.telemetry_client = None
        self._collect_data = False

        self._telemetry_receiving_thread = threading.Thread(target=self._receive_telemetry, daemon=True)
        self._telemetry_receiving_thread.start()

        self._graph_data = {}

        pyros_client_app.add_on_connected_subscriber(self._connect_telemetry)

    def _connect_telemetry(self):
        host, port = pyros.get_connection_details()
        if self.telemetry_client is None or self.telemetry_client.host != host:
            self._setup_telemetry_client(host, 1860)
        self._collect_data = True
        self.start_stop_collecting_panel.components[0].set_visible(True)
        self.start_stop_collecting_panel.components[1].set_visible(False)

    def _setup_telemetry_client(self, host, port):
        self.start_stop_collecting_panel.components[0].set_visible(True)
        self.start_stop_collecting_panel.components[1].set_visible(False)
        self.start_stop_balancing_panel.components[0].set_visible(False)
        self.start_stop_balancing_panel.components[1].set_visible(True)

        self.telemetry_client = CachingSocketTelemetryClient(host=host, port=port)
        self.telemetry_client.start()
        self.telemetry_client.socket.settimeout(2)
        self._graph_data["gdx"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'gdx', 50, -50, auto_scale=True)
        self._graph_data["gdy"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'gdy', 50, -50, auto_scale=True)
        self._graph_data["gdz"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'gdz', 50, -50, auto_scale=True)
        self._graph_data["gx"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'gx', 90.0, -90.0, auto_scale=True)
        self._graph_data["gy"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'gy', 90.0, -90.0, auto_scale=True)
        self._graph_data["gz"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'gz', 90.0, -90.0, auto_scale=True)
        self._graph_data["adx"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'adx', 50, -50, auto_scale=True)
        self._graph_data["ady"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'ady', 50, -50, auto_scale=True)
        self._graph_data["adz"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'adz', 50, -50, auto_scale=True)
        self._graph_data["ax"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'ax', 1.5, -1.5)
        self._graph_data["ay"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'ay', 1.5, -1.5)
        self._graph_data["az"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'az', 1.5, -1.5)
        self._graph_data["data_points"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'data_points', 10, 0)
        self._graph_data["status"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'status', 255, 0)
        self._graph_data["fifo_status"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'fifo_status', 255, 0)
        self._graph_data["apitch"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'apitch', 180.0, -180.0)
        self._graph_data["aroll"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'aroll', 180, -180.0)
        self._graph_data["ayaw"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'ayaw', 180, -180.0)
        self._graph_data["lw"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'lw', 360, 0.0)
        self._graph_data["rw"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'rw', 360, 0.0)
        self._graph_data["cx"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'cx', 180, -180.0)
        self._graph_data["cy"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'cy', 180, -180.0)
        self._graph_data["cz"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'cz', 180, -180.0)
        self._graph_data["pi_p"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_p', 0.1, -0.1, auto_scale=True)
        self._graph_data["pi_i"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_i', 0.1, -0.1, auto_scale=True)
        self._graph_data["pi_d"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_d', 0.1, -0.1, auto_scale=True)
        self._graph_data["pi_pg"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_pg', 0.1, -0.1, auto_scale=True)
        self._graph_data["pi_ig"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_ig', 0.1, -0.1, auto_scale=True)
        self._graph_data["pi_dg"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_dg', 0.1, -0.1, auto_scale=True)
        self._graph_data["pi_dt"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_dt', 0.001, -0.001, auto_scale=True)
        self._graph_data["pi_o"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'pi_o', 0.1, -0.1, auto_scale=True)
        self._graph_data["out"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'out', 0.1, -0.1, auto_scale=True)
        self._graph_data["pi_slo"] = ChangedSingleTelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'out', 10.0, -10.0, lambda x: x * 100, auto_scale=True)

        self.sensors_graphs_panel.main_graph.set_graph_data(self._graph_data['cy'])
        self.sensors_graphs_panel.graphs[0][0].set_graph_data(self._graph_data['gdy'])
        self.sensors_graphs_panel.graphs[0][1].set_graph_data(self._graph_data['gy'])
        self.sensors_graphs_panel.graphs[0][2].set_graph_data(self._graph_data['data_points'])
        self.sensors_graphs_panel.graphs[0][3].set_graph_data(self._graph_data['cy'])

        self.sensors_graphs_panel.graphs[1][0].set_graph_data(self._graph_data['adx'])
        self.sensors_graphs_panel.graphs[1][1].set_graph_data(self._graph_data['ady'])
        self.sensors_graphs_panel.graphs[1][2].set_graph_data(self._graph_data['adz'])
        self.sensors_graphs_panel.graphs[1][3].set_graph_data(self._graph_data['aroll'])

        self.sensors_graphs_panel.graphs[2][0].set_graph_data(self._graph_data['lw'])
        self.sensors_graphs_panel.graphs[2][1].set_graph_data(self._graph_data['rw'])
        self.sensors_graphs_panel.graphs[2][2].set_graph_data(self._graph_data['az'])
        self.sensors_graphs_panel.graphs[2][3].set_graph_data(self._graph_data['apitch'])

        self.pid_graphs_panel.main_graph.set_graph_data(self._graph_data['cy'])
        self.pid_graphs_panel.graphs[0][0].set_graph_data(self._graph_data['pi_pg'])
        self.pid_graphs_panel.graphs[0][1].set_graph_data(self._graph_data['pi_ig'])
        self.pid_graphs_panel.graphs[0][2].set_graph_data(self._graph_data['pi_dg'])

        self.pid_graphs_panel.graphs[1][0].set_graph_data(self._graph_data['pi_dt'])
        self.pid_graphs_panel.graphs[1][1].set_graph_data(self._graph_data['pi_dt'])
        self.pid_graphs_panel.graphs[1][2].set_graph_data(self._graph_data['pi_slo'])

        self.sensors_graphs_panel.add_value_overlay(0, 3, "{0.1f}")
        self.sensors_graphs_panel.add_value_overlay(1, 3, "{0.1f}")
        self.sensors_graphs_panel.add_value_overlay(2, 0, "{0.1f}")
        self.sensors_graphs_panel.add_value_overlay(2, 1, "{0.1f}")
        self.sensors_graphs_panel.add_value_overlay(2, 3, "{0.1f}")
        self.sensors_graphs_panel.add_value_overlay(2, 3, "{0.1f}")



    def key_down(self, key):
        if self.pyros_client_app.key_down(key):
            pass
        # elif key == pygame.K_s:
        # else:
        #     pyros_support_ui.gcc.handle_connect_key_down(key)

    def key_up(self, key):
        if self.pyros_client_app.key_up(key):
            pass

    def redefine_rect(self, rect):
        self.rect = rect
        self.commands.redefine_rect(Rect(rect.x, rect.y, rect.width, 30))
        self.graphs_panel.redefine_rect(Rect(rect.x, self.commands.rect.bottom + 5, rect.width - 300 - 5, rect.bottom - self.commands.rect.bottom - 5))
        self.values_panel.redefine_rect(
            Rect(self.sensors_graphs_panel.rect.right + 5, self.commands.rect.bottom + 5, rect.right - self.sensors_graphs_panel.rect.right - 5, rect.bottom - self.commands.rect.bottom - 5))

    def start_balancing(self, *_args):
        pyros.publish("balancing/start", "")
        if pyros.is_connected():
            self.start_stop_balancing_panel.components[0].set_visible(False)
            self.start_stop_balancing_panel.components[1].set_visible(True)

    def stop_balancing(self, *_args):
        pyros.publish("balancing/stop", "")
        if pyros.is_connected():
            self.start_stop_balancing_panel.components[0].set_visible(True)
            self.start_stop_balancing_panel.components[1].set_visible(False)

    def manual(self, *_args):
        pyros.publish("manual", "{0:.2f}".format(self.values_panel.manual))
        if pyros.is_connected():
            self.start_stop_balancing_panel.components[0].set_visible(False)
            self.start_stop_balancing_panel.components[1].set_visible(True)

    @staticmethod
    def calibrate(*_args):
        pyros.publish("balancing/calibrate", "all")

    def stop_collecting(self, *_args):
        self._collect_data = False
        self.telemetry_client.stop()
        self.start_stop_collecting_panel.components[0].set_visible(False)
        self.start_stop_collecting_panel.components[1].set_visible(True)

    def continue_collecting(self, *_args):
        if pyros.is_connected():
            if self.telemetry_client is None and pyros.is_connected():
                host, port = pyros.get_connection_details()
                self._setup_telemetry_client(host, 1860)
            else:
                self.telemetry_client.start()
            self._collect_data = True
            self.start_stop_collecting_panel.components[0].set_visible(True)
            self.start_stop_collecting_panel.components[1].set_visible(False)

    def show_pid(self, *_args):
        self.sensors_pid_switch_panel.components[0].set_visible(False)
        self.sensors_pid_switch_panel.components[1].set_visible(True)
        self.graphs_panel.select_card("pid")

    def show_sensors(self, *_args):
        self.sensors_pid_switch_panel.components[0].set_visible(True)
        self.sensors_pid_switch_panel.components[1].set_visible(False)
        self.graphs_panel.select_card("sensors")

    def pause_graph(self, *_args):
        self.pause_graph_panel.components[0].set_visible(True)
        self.pause_graph_panel.components[1].set_visible(False)
        self.graph_controller.pause()

    def resume_graph(self, *_args):
        self.pause_graph_panel.components[0].set_visible(False)
        self.pause_graph_panel.components[1].set_visible(True)
        self.graph_controller.resume()

    def clear_graph(self, *_args):
        def clear_data(stream):
            self.telemetry_client.trim(stream, time.time())

        self.telemetry_client.get_stream_definition("balance-data", clear_data)

    def save_graph_all(self, *_args):

        with open("logs.csv", "wt") as file:
            def write_data(records):
                for record in records:
                    file.write(",".join([str(f) for f in record]) + "\n")

            def write_header(stream):
                file.write("timestamp," + ",".join(f.name for f in stream.fields) + "\n")
                self.telemetry_client.retrieve(stream, 0, time.time(), write_data)

            self.telemetry_client.get_stream_definition("balance-data", write_header)

    def save_graph(self, *_args):

        fields_to_save = ["lw", "rw", "cy", "pi_p", "pi_i", "pi_d", "pi_pg", "pi_ig", "pi_dg", "pi_dt", "pi_o", "out"]
        field_indexes = [0]

        with open("logs.csv", "wt") as file:
            def write_data(records):
                for record in records:
                    file.write(",".join(str(record[i]) for i in field_indexes) + "\n")

            def write_header(stream):
                filtered_fields = [f.name for f in stream.fields if f.name in fields_to_save]
                field_indexes.extend(i + 1 for i in range(len(stream.fields)) if stream.fields[i].name in filtered_fields)
                print(f" fields {filtered_fields} and indexes {field_indexes}")
                file.write("timestamp," + ",".join(f for f in filtered_fields) + "\n")
                self.telemetry_client.retrieve(stream, 0, time.time(), write_data)

            self.telemetry_client.get_stream_definition("balance-data", write_header)

    def _receive_telemetry(self):
        had_exception = False
        while True:
            try:
                if self._collect_data:
                    self.telemetry_client.process_incoming_data()
                had_exception = False
            except Exception as ex:
                if not had_exception:
                    print("ERROR: " + str(ex) + "\n" + ''.join(traceback.format_tb(ex.__traceback__)))
                    had_exception = True


balancing_rover_telemetry = BalancingRoverTelemetry(ui_factory, pyros_client_app)
pyros_client_app.set_content(balancing_rover_telemetry)
pyros_client_app.start_discovery_in_background()

while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            pygame.quit()
            sys.exit()

        ui_adapter.process_event(event)
        if event.type == pygame.KEYDOWN:
            balancing_rover_telemetry.key_down(event.key)
        if event.type == pygame.KEYUP:
            balancing_rover_telemetry.key_up(event.key)

    pyros.loop(0.03)

    ui_adapter.draw(ui_adapter.get_screen())

    ui_adapter.frame_end()
