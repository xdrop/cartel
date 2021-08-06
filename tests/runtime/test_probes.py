import re
from time import sleep

import pytest

from runtime.shim import (
    eventual_exit_shim,
    exit_toggle_shim,
    net_listener_service_shim,
    service_shim,
)


@pytest.mark.slow
def test_wait_for_network_readiness_probe(cartel):
    # GIVEN
    svc1 = net_listener_service_shim(delay=6)

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        readiness_probe:
            type: net
            host: localhost
            port: {svc1.port}
            retries: 2
        """
    )

    # WHEN/THEN
    with cartel.client_cmd_tty(["deploy", "svc-1"]) as tty:
        # should not be ready before <5 seconds
        assert not tty.expect(pattern="Deployed modules", timeout=5)
        # should be ready by 10 seconds
        assert tty.expect(pattern="Deployed modules", timeout=5)


@pytest.mark.slow
def test_wait_for_network_readiness_probe_exceeds_retries(cartel):
    # GIVEN
    svc1 = net_listener_service_shim(delay=20)

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        readiness_probe:
            type: net
            host: localhost
            port: {svc1.port}
            retries: 1
        """
    )

    # WHEN
    out = cartel.client_cmd(["deploy", "svc-1"], timeout=5)

    # THEN
    assert (
        "Error: The service did not complete its readiness"
        " probe checks in time." in out
    )
    assert "Check the logs for more details." in out


@pytest.mark.slow
@pytest.mark.parametrize("cmd_line_type", [("shell"), ("command")])
def test_wait_for_exec_readiness_probe(cmd_line_type, cartel):
    # GIVEN
    svc = service_shim()
    probe = eventual_exit_shim(delay=6)

    cmd_line = getattr(probe, cmd_line_type)

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc.shell}
        readiness_probe:
            type: exec
            {cmd_line_type}: {cmd_line}
            retries: 2
        """
    )

    # # WHEN/THEN
    with cartel.client_cmd_tty(["deploy", "svc-1"]) as tty:
        # should not be ready before <5 seconds
        assert not tty.expect(pattern="Deployed modules", timeout=5)
        # should be ready by 10 seconds
        assert tty.expect(pattern="Deployed modules", timeout=5)


@pytest.mark.slow
def test_wait_for_exec_readiness_exceed_retries(cartel):
    # GIVEN
    svc = service_shim()
    probe = eventual_exit_shim(delay=20)

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc.shell}
        readiness_probe:
            type: exec
            command: {probe.command}
            retries: 1
        """
    )

    # WHEN
    out = cartel.client_cmd(["deploy", "svc-1"], timeout=5)

    # THEN
    assert (
        "Error: The service did not complete its readiness"
        " probe checks in time." in out
    )
    assert "Check the logs for more details." in out


@pytest.mark.slow
def test_wait_for_log_line_readiness_probe(cartel):
    # GIVEN
    svc = service_shim(delay=6, msg="pass")

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc.shell}
        readiness_probe:
            type: log_line
            line_regex: pass
            retries: 2
        """
    )

    # WHEN/THEN
    with cartel.client_cmd_tty(["deploy", "svc-1"]) as tty:
        # should not be ready before <5 seconds
        assert not tty.expect(pattern="Deployed modules", timeout=5)
        # should be ready by 10 seconds
        assert tty.expect(pattern="Deployed modules", timeout=5)


@pytest.mark.slow
def test_wait_for_log_line_readiness_exceed_retries(cartel):
    # GIVEN
    svc = service_shim(delay=6, msg="pass")

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc.shell}
        readiness_probe:
            type: log_line
            line_regex: pass
            retries: 1
        """
    )

    # WHEN
    out = cartel.client_cmd(["deploy", "svc-1"], timeout=5)

    # THEN
    assert (
        "Error: The service did not complete its readiness"
        " probe checks in time." in out
    )
    assert "Check the logs for more details." in out


@pytest.mark.slow
def test_liveness_probe(cartel):
    # GIVEN
    svc = service_shim()
    probe = exit_toggle_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc.shell}
        liveness_probe:
            type: exec
            shell: {probe.shell}
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "svc-1"])

    # THEN
    ps_output = cartel.client_cmd(["ps"])
    assert re.findall(r"^\d+\s+svc-1\s+-\s+running\s+.*", ps_output, re.M)

    # WHEN
    probe.toggle()
    sleep(6)

    # THEN
    ps_output = cartel.client_cmd(["ps"])
    assert re.findall(r"^\d+\s+svc-1\s+healthy\s+running\s+.*", ps_output, re.M)

    # WHEN
    probe.toggle()
    sleep(6)

    # THEN
    ps_output = cartel.client_cmd(["ps"])
    assert re.findall(r"^\d+\s+svc-1\s+failing\s+running\s+.*", ps_output, re.M)
