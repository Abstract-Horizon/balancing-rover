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

# from functools import partial

from pygame import Rect

from pyros_support_ui import PyrosClientApp
from pyros_support_ui.components import UIAdapter, Collection, LeftRightLayout, UiHint
from pyros_support_ui.pygamehelper import load_font, load_image

from pyros_support_ui.box_blue_sf_theme import BoxBlueSFThemeFactory
# from pyros_support_ui.flat_theme import FlatThemeFactory

from graph_component import Graph, TelemetryGraphData

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
    def __init__(self, rows, columns):
        super(GraphsPanel, self).__init__(None)
        self.rows = rows
        self.columns = columns
        self.main_graph = Graph(None, ui_factory)
        self.add_component(self.main_graph)
        self.graphs = []
        for i in range(rows):
            self.graphs.append([])
            for c in range(columns):
                graph = Graph(None, ui_factory)
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


class BalancingRoverTelemetry(Collection):
    def __init__(self, _ui_factory, _pyros_client_app):
        super(BalancingRoverTelemetry, self).__init__(None)
        self.pyros_client_app = _pyros_client_app
        self.commands = CommandsPanel()
        self.add_component(self.commands)
        self.graphs_panel = GraphsPanel(3, 4)
        self.add_component(self.graphs_panel)

        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Start", on_click=self.start_balancing))
        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Stop", on_click=self.stop_balancing, hint=UiHint.WARNING))
        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 100, 0), "Calibrate", on_click=self.calibrate))

        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Stop Collecting", on_click=self.stop_collecting))
        self.commands.add_component(_ui_factory.text_button(Rect(0, 0, 200, 0), "Continue Collecting", on_click=self.continue_collecting))

        self.telemetry_client = None
        self._collect_data = False

        self._telemetry_receiving_thread = threading.Thread(target=self._receive_telemetry, daemon=True)
        self._telemetry_receiving_thread.start()

        self._graph_data = {}

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
        self.graphs_panel.redefine_rect(Rect(rect.x, self.commands.rect.bottom + 5, rect.width, rect.bottom - self.commands.rect.bottom - 5))

    def _setup_telemetry_client(self, host, port):
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
        self._graph_data["cx"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'cx', 180, -180.0)
        self._graph_data["cy"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'cy', 180, -180.0)
        self._graph_data["cz"] = TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'cz', 180, -180.0)

    def start_balancing(self, *_args):
        pyros.publish("balancing/start", "")
        if pyros.is_connected():
            if self.telemetry_client is None:
                host, port = pyros.get_connection_details()
                self._setup_telemetry_client(host, 1860)
                # self.telemetry_client = CachingSocketTelemetryClient(host=host, port=1860)
                # self.telemetry_client.start()
                # self.telemetry_client.socket.settimeout(2)
            else:
                host, port = pyros.get_connection_details()
                if self.telemetry_client.host != host:
                    self._setup_telemetry_client(host, 1860)
                    # self.telemetry_client = CachingSocketTelemetryClient(host=host, port=1860)
                    # self.telemetry_client.start()
                    # self.telemetry_client.socket.settimeout(2)
            self._collect_data = True

            self.graphs_panel.main_graph.set_graph_data(self._graph_data['cx'])
            # self.graphs_panel.graph_01.set_graph_data(TelemetryGraphData(self.telemetry_client, self.telemetry_client.streams['balance-data'], 'gdx', 32767, -32768))
            self.graphs_panel.graphs[0][0].set_graph_data(self._graph_data['gdx'])
            self.graphs_panel.graphs[0][1].set_graph_data(self._graph_data['gx'])
            self.graphs_panel.graphs[0][2].set_graph_data(self._graph_data['data_points'])
            self.graphs_panel.graphs[0][3].set_graph_data(self._graph_data['cx'])

            self.graphs_panel.graphs[1][0].set_graph_data(self._graph_data['adx'])
            self.graphs_panel.graphs[1][1].set_graph_data(self._graph_data['ady'])
            self.graphs_panel.graphs[1][2].set_graph_data(self._graph_data['adz'])
            self.graphs_panel.graphs[1][3].set_graph_data(self._graph_data['aroll'])

            self.graphs_panel.graphs[2][0].set_graph_data(self._graph_data['ax'])
            self.graphs_panel.graphs[2][1].set_graph_data(self._graph_data['ay'])
            self.graphs_panel.graphs[2][2].set_graph_data(self._graph_data['az'])
            self.graphs_panel.graphs[2][3].set_graph_data(self._graph_data['apitch'])
        else:
            self._collect_data = False

    @staticmethod
    def stop_balancing(*_args):
        pyros.publish("balancing/stop", "")

    @staticmethod
    def calibrate(*_args):
        pyros.publish("balancing/calibrate", "all")

    def stop_collecting(self, *_args):
        # for k in self._graph_data:
        #     graph_data = self._graph_data[k]
        #     graph_data.collect = False
        self.telemetry_client.stop()

    def continue_collecting(self, *_args):
        # for k in self._graph_data:
        #     graph_data = self._graph_data[k]
        #     graph_data.collect = True
        if self.telemetry_client is None and pyros.is_connected():
            host, port = pyros.get_connection_details()
            self._setup_telemetry_client(host, 1860)

        else:
            self.telemetry_client.start()

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
