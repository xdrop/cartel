import subprocess
import tempfile
import textwrap

import pytest

from runtime.client import client_cmd, client_cmd_tty
from runtime.paths import debug_binaries_path
from runtime.helpers import kill_all_shims


def pytest_addoption(parser):
    parser.addoption(
        "--runslow", action="store_true", default=False, help="run slow tests"
    )


def pytest_configure(config):
    config.addinivalue_line("markers", "slow: mark test as slow to run")


def pytest_collection_modifyitems(config, items):
    if config.getoption("--runslow"):
        return
    skip_slow = pytest.mark.skip(reason="need --runslow option to run")
    for item in items:
        if "slow" in item.keywords:
            item.add_marker(skip_slow)


class CartelTestHelper:
    def __init__(self, proc):
        self.proc = proc
        self.definitions_file = tempfile.NamedTemporaryFile()

    def definitions(self, definitions):
        self._definitions_file_content = textwrap.dedent(definitions)
        self.definitions_file.write(
            self._definitions_file_content.encode("utf-8")
        )
        self.definitions_file.flush()

    @property
    def definitions_file_content(self):
        return self._definitions_file_content

    @property
    def definition_file_path(self):
        return str(self.definitions_file.name)

    def client_cmd(self, args, *rargs, **kwargs):
        args = ["-f", self.definition_file_path, *args]
        return client_cmd(args, *rargs, **kwargs)

    def client_cmd_tty(self, args):
        args = ["-f", self.definition_file_path, *args]
        return client_cmd_tty(args)

    def cleanup(self):
        self.definitions_file.close()


@pytest.fixture
def cartel():
    debug_binaries = debug_binaries_path()
    daemon_path = debug_binaries.joinpath("daemon")
    with subprocess.Popen([str(daemon_path)]) as proc:
        cartel = CartelTestHelper(proc)
        yield cartel
        proc.terminate()
        cartel.cleanup()


@pytest.fixture(autouse=True)
def cleanup():
    yield
    kill_all_shims()
