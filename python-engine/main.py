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

import os
import sys
import getopt
import locale
import logging

import gi
gi.require_version('IBus', '1.0')
from gi.repository import IBus
from gi.repository import GLib

import factory

class IMApp:
    def __init__(self, exec_by_ibus):
        # Setup logging
        logging.basicConfig(
            level=logging.DEBUG,
            format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
        )
        self.logger = logging.getLogger('thaime-python')
        self.logger.info("Starting Thaime Python IME")
        
        # Create IBus component
        self.__component = IBus.Component(
            name="org.freedesktop.IBus.ThaimePython",
            description="Thaime Python Component",
            version="0.1.0",
            license="GPL-3.0-or-later",
            author="mimocha <chawit.leosrisook@outlook.com>",
            homepage="https://github.com/mimocha/thaime",
            command_line="",
            textdomain="thaime"
        )
        
        # Add engine to component
        engine = IBus.EngineDesc(
            name="thaime-python",
            longname="Thai (thaime-python)",
            description="Thai Input Method Engine (Python)",
            language="th",
            license="GPL-3.0-or-later",
            author="mimocha <chawit.leosrisook@outlook.com>",
            icon="",
            layout="th",
            rank=99
        )
        self.__component.add_engine(engine)
        
        # Create main loop and bus
        self.__mainloop = GLib.MainLoop()
        self.__bus = IBus.Bus()
        self.__bus.connect("disconnected", self.__bus_disconnected_cb)
        
        # Create engine factory
        self.__factory = factory.EngineFactory(self.__bus)
        
        if exec_by_ibus:
            self.__bus.request_name("org.freedesktop.IBus.ThaimePython", 0)
        else:
            self.__bus.register_component(self.__component)
            
        self.logger.info("Thaime Python IME initialized")

    def run(self):
        self.logger.info("Running main loop")
        self.__mainloop.run()

    def __bus_disconnected_cb(self, bus):
        self.logger.info("Bus disconnected, quitting")
        self.__mainloop.quit()


def launch_engine(exec_by_ibus):
    app = IMApp(exec_by_ibus)
    app.run()

def print_help(out, v=0):
    print("-i, --ibus             executed by ibus.", file=out)
    print("-h, --help             show this message.", file=out)
    print("-d, --daemonize        daemonize ibus", file=out)
    sys.exit(v)

def main():
    try:
        locale.setlocale(locale.LC_ALL, "")
    except:
        pass

    exec_by_ibus = False
    daemonize = False

    shortopt = "ihd"
    longopt = ["ibus", "help", "daemonize"]

    try:
        opts, args = getopt.getopt(sys.argv[1:], shortopt, longopt)
    except getopt.GetoptError as err:
        print(str(err), file=sys.stderr)
        print_help(sys.stderr, 1)

    for o, a in opts:
        if o in ("-h", "--help"):
            print_help(sys.stdout)
        elif o in ("-d", "--daemonize"):
            daemonize = True
        elif o in ("-i", "--ibus"):
            exec_by_ibus = True
        else:
            print("Unknown argument: %s" % o, file=sys.stderr)
            print_help(sys.stderr, 1)

    if daemonize:
        if os.fork():
            sys.exit()

    launch_engine(exec_by_ibus)

if __name__ == "__main__":
    main()