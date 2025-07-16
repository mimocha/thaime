#!/usr/bin/env python3
# -*- coding: utf-8 -*-
#
# thaime - Thai Input Method Engine
#
# Copyright (c) 2024 mimocha <chawit.leosrisook@outlook.com>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 3, or (at your option)
# any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 675 Mass Ave, Cambridge, MA 02139, USA.

import gi
gi.require_version('IBus', '1.0')
from gi.repository import IBus
import logging

import engine


class EngineFactory(IBus.Factory):
    def __init__(self, bus):
        super(EngineFactory, self).__init__(
            object_path=IBus.PATH_FACTORY,
            connection=bus.get_connection()
        )
        self.__bus = bus
        self.__id = 0
        self.logger = logging.getLogger('thaime-python.factory')
        self.logger.info("Engine factory created")

    def do_create_engine(self, engine_name):
        self.logger.info(f"Creating engine: {engine_name}")
        if engine_name == "thaime-python":
            self.__id += 1
            engine_path = "/org/freedesktop/IBus/ThaimePython/Engine/%d" % self.__id
            self.logger.info(f"Creating engine instance with path: {engine_path}")
            return engine.Engine(self.__bus, engine_path)
        
        self.logger.warning(f"Unknown engine name: {engine_name}")
        return None