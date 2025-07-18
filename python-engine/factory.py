import logging

import gi
gi.require_version('IBus', '1.0')

import engine
from gi.repository import IBus


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