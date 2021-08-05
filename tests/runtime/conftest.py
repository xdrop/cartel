import subprocess
from pathlib import Path

import pytest


def target_dir_path():
    cwd = Path(__file__).resolve()
    target_dir = cwd.parent.parent.parent.joinpath("target")
    return target_dir


def debug_binaries_path():
    target_dir = target_dir_path()
    return target_dir.joinpath("debug")


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


@pytest.fixture
def daemon():
    debug_binaries = debug_binaries_path()
    daemon_path = debug_binaries.joinpath("daemon")
    with subprocess.Popen([str(daemon_path)]) as proc:
        yield
        proc.terminate()
