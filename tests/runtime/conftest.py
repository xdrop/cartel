import subprocess

import pytest

from runtime.helpers import debug_binaries_path


@pytest.fixture
def daemon():
    debug_binaries = debug_binaries_path()
    daemon_path = debug_binaries.joinpath("daemon")
    with subprocess.Popen([str(daemon_path)]) as proc:
        yield
        proc.kill()
