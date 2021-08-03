from runtime.helpers import process_running


def test_setup(daemon):
    assert process_running("daemon")
