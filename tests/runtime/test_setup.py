from runtime.helpers import process_running


def test_setup(cartel):
    assert process_running("daemon")
