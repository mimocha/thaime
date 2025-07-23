import logging

import gi
gi.require_version('IBus', '1.0')

import engine
from gi.repository import IBus


class EngineFactory(IBus.Factory):
    def __init__(self, bus):
        self.__bus = bus
        super(EngineFactory, self).__init__(self.__bus)

        self.__id = 0
        self.logger = logging.getLogger('thaime.factory')
        self.logger.info("Engine factory created")

    def do_create_engine(self, engine_name):
        self.logger.info(f"Creating engine: {engine_name}")
        self.__id += 1
        engine_path = f"/org/freedesktop/IBus/Thaime/Engine/{self.__id}"
        self.logger.info(f"Creating engine instance with path: {engine_path}")

        if engine_name == "thaime":
            return engine.Engine(self.__bus, engine_path)
        else:
            self.logger.error(f"Unknown engine name: {engine_name}")
            raise NotImplementedError(f"Unknown engine name: {engine_name}")