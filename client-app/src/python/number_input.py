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

from pygame import Rect

from pyros_support_ui.components import Component, Collection, ALIGNMENT, UiHint, LeftRightLayout


class NumberInputComponent(Collection):
    def __init__(self, rect, ui_factory, getter_setter_pair, name, button_font=None, value_font=None):
        super(NumberInputComponent, self).__init__(rect, layout=LeftRightLayout(margin=5))
        self.getter = getter_setter_pair[0]
        self.setter = getter_setter_pair[1]
        self.name = name
        self.button_font = button_font if button_font is not None else ui_factory.get_font()
        self.value_font = value_font if value_font is not None else ui_factory.get_font()
        self.height = 40 if rect is None or rect.height == 0 else rect.height

        self.widths = [self.button_font.size(w)[0] for w in ["<", "<<", "<<<", ">>>", ">>", ">"]]
        self.margin = 3

        self.add_component(ui_factory.text_button(Rect(rect.x, rect.y, self.widths[0], self.height), "<", self.on_click_minus1, font=self.button_font, hint=UiHint.NO_DECORATION))
        self.add_component(ui_factory.text_button(Rect(rect.x + 34, rect.y, self.widths[1], self.height), "<<", self.on_click_minus01, font=self.button_font, hint=UiHint.NO_DECORATION))
        self.add_component(ui_factory.text_button(Rect(rect.x + 34 * 2, rect.y, self.widths[2], self.height), "<<<", self.on_click_minus001, font=self.button_font, hint=UiHint.NO_DECORATION))
        self.add_component(Component(Rect(0, 0, 5, self.height)))
        self.add_component(ui_factory.label(Rect(rect.x + 112, rect.y, 30, self.height), self.name, v_alignment=ALIGNMENT.MIDDLE, font=self.value_font))

        self.left = ui_factory.label(Rect(rect.x + 100, rect.y, 64, self.height), '', h_alignment=ALIGNMENT.RIGHT, v_alignment=ALIGNMENT.MIDDLE)
        self.right = ui_factory.label(Rect(rect.x + 164, rect.y, 30, self.height), '', h_alignment=ALIGNMENT.LEFT, v_alignment=ALIGNMENT.MIDDLE)
        self.add_component(self.left)
        self.add_component(self.right)

        self.add_component(ui_factory.text_button(Rect(rect.right - 30 - 34 * 2, rect.y, self.widths[3], self.height), ">>>", self.on_click_plus001, font=self.button_font, hint=UiHint.NO_DECORATION))
        self.add_component(ui_factory.text_button(Rect(rect.right - 30 - 34, rect.y, self.widths[4], self.height), ">>", self.on_click_plus01, font=self.button_font, hint=UiHint.NO_DECORATION))
        self.add_component(ui_factory.text_button(Rect(rect.right - 30, rect.y, self.widths[5], self.height), ">", self.on_click_plus1, font=self.button_font, hint=UiHint.NO_DECORATION))
        # self.redefine_rect(rect)

    def redefine_rect(self, rect):
        super(NumberInputComponent, self).redefine_rect(rect)
        # self.rect = rect
        #
        # self.components[3].redefine_rect(Rect(rect.x, rect.y, self.widths[0], self.height))
        # self.components[4].redefine_rect(Rect(self.components[3].rect.right + self.margin, rect.y, self.widths[1], self.height))
        # self.components[5].redefine_rect(Rect(self.components[4].rect.right + self.margin, rect.y, self.widths[2], self.height))
        #
        # self.components[1].redefine_rect(Rect(self.components[5].rect.right + self.margin, rect.y, 64, self.height))
        # self.components[0].redefine_rect(Rect(rect.x + 112, rect.y, 30, self.height))
        # self.components[2].redefine_rect(Rect(rect.x + 164, rect.y, 30, self.height))
        #
        # self.components[6].redefine_rect(Rect(rect.right - 30 - 34 * 2, rect.y, self.widths[3], self.height))
        # self.components[7].redefine_rect(Rect(rect.right - 30 - 34, rect.y, self.widths[4], self.height))
        # self.components[8].redefine_rect(Rect(rect.right - 30, rect.y, self.widths[5], self.height))

    def draw(self, surface):
        value = self.getter()

        s = "{0:.2f}".format(value)
        i = s.index('.')
        left = s[0:i + 1]
        right = s[i + 1:]
        self.left.set_text(left)
        self.right.set_text(right)

        super(NumberInputComponent, self).draw(surface)

    def on_click_plus1(self, button, pos):
        value = self.getter()
        value += 1.0
        self.setter(value)

    def on_click_minus1(self, button, pos):
        value = self.getter()
        value -= 1.0
        self.setter(value)

    def on_click_plus01(self, button, pos):
        value = self.getter()
        value += 0.1
        self.setter(value)

    def on_click_minus01(self, button, pos):
        value = self.getter()
        value -= 0.1
        self.setter(value)

    def on_click_plus001(self, button, pos):
        value = self.getter()
        value += 0.01
        self.setter(value)

    def on_click_minus001(self, button, pos):
        value = self.getter()
        value -= 0.01
        self.setter(value)
