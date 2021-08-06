from runtime.helpers import find_pid, run_service
from runtime.shim import service_shim


def test_stops_single_service(cartel):
    # GIVEN
    svc = run_service("stop-test-1")

    pid = find_pid(svc.process_name)

    # WHEN
    assert pid
    out = cartel.client_cmd(["stop", "stop-test-1"])

    # THEN

    assert "Stopping stop-test-1 (Stopped)" in out
    assert not find_pid(svc.process_name, pid=pid)


def test_stops_multiple_services(cartel):
    # GIVEN
    svc1 = run_service("stop-test-1")
    svc2 = run_service("stop-test-2")

    pid1 = find_pid(svc1.process_name)
    pid2 = find_pid(svc2.process_name)

    # WHEN
    assert pid1
    assert pid2
    out = cartel.client_cmd(["stop", "stop-test-1", "stop-test-2"])

    # THEN
    assert "Stopping stop-test-1 (Stopped)" in out
    assert "Stopping stop-test-2 (Stopped)" in out
    assert not find_pid(svc1.process_name, pid=pid1)
    assert not find_pid(svc2.process_name, pid=pid2)


def test_stops_with_sigterm_if_specified(cartel):
    # GIVEN
    svc = service_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: stop-test-sigterm-1
        shell: {svc.shell}
        termination_signal: TERM
        """
    )
    cartel.client_cmd(["deploy", "-f", "stop-test-sigterm-1"])

    pid = find_pid(svc.process_name)

    # WHEN
    assert pid
    out = cartel.client_cmd(["stop", "stop-test-sigterm-1"])

    # THEN

    assert "Stopping stop-test-sigterm-1 (Stopped)" in out
    assert not find_pid(svc.process_name)
    assert svc.signal == "SIGTERM"


def test_stops_with_sigint_if_specified(cartel):
    # GIVEN
    svc = service_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: stop-test-sigint-1
        shell: {svc.shell}
        termination_signal: INT
        """
    )
    cartel.client_cmd(["deploy", "-f", "stop-test-sigint-1"])

    pid = find_pid(svc.process_name)

    # WHEN
    assert pid
    out = cartel.client_cmd(["stop", "stop-test-sigint-1"])

    # THEN

    assert "Stopping stop-test-sigint-1 (Stopped)" in out
    assert not find_pid(svc.process_name)
    assert svc.signal == "SIGINT"
