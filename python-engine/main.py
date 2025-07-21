import getopt
import locale
import logging
import os
import sys

import gi
gi.require_version('IBus', '1.0')

import factory
from gi.repository import GLib, IBus


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
            description="Thaime Python Engine Collection",
            version="0.1.1",
            license="GPL-3.0-or-later",
            author="mimocha <chawit.leosrisook@outlook.com>",
            command_line="",
            textdomain="thaime"
        )
        
        # Add Latin engine
        latin_engine = IBus.EngineDesc(
            name="thaime-python-latin",
            longname="Latin (thaime-python)",
            description="Latin Input Method Engine - Standard QWERTY",
            language="en",
            license="GPL-3.0-or-later",
            author="mimocha <chawit.leosrisook@outlook.com>",
            icon="",
            layout="us",
            rank=1
        )
        self.__component.add_engine(latin_engine)
        
        # Add Thai-Kedmanee engine
        kedmanee_engine = IBus.EngineDesc(
            name="thaime-python-kedmanee",
            longname="Thai Kedmanee (thaime-python)",
            description="Thai Kedmanee Input Method Engine",
            language="th",
            license="GPL-3.0-or-later",
            author="mimocha <chawit.leosrisook@outlook.com>",
            icon="",
            layout="th",
            rank=2
        )
        self.__component.add_engine(kedmanee_engine)
        
        # Add Thaime engine
        thaime_engine = IBus.EngineDesc(
            name="thaime-python-thaime",
            longname="Thaime (thaime-python)",
            description="Thaime Input Method Engine - Latin to Thai conversion (experimental)",
            language="th",
            license="GPL-3.0-or-later",
            author="mimocha <chawit.leosrisook@outlook.com>",
            icon="",
            layout="us",
            rank=3
        )
        self.__component.add_engine(thaime_engine)
        
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