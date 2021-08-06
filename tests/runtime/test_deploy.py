import re
from time import sleep

import pytest

from runtime.shim import (
    eventual_exit_shim,
    exit_toggle_shim,
    net_listener_service_shim,
    service_shim,
    task_shim,
)


def test_deploy_single_service(cartel):
    # GIVEN
    svc = service_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        """
    )

    # WHEN
    out = cartel.client_cmd(["deploy", "svc"])

    # THEN
    assert "Deploying svc (Deployed)" in out
    assert 'Deployed modules: ["svc"]' in out
    assert svc.ran_once()


def test_deploy_tasks_before_service(cartel):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        dependencies: [task-1, task-2, task-3]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        ---
        kind: Task
        name: task-3
        shell: {tsk3.shell}
        """
    )

    # WHEN
    out = cartel.client_cmd(["deploy", "svc-1"])

    # THEN
    assert "Deploying svc-1 (Deployed)" in out
    assert "Running task task-1 (Done)" in out
    assert "Running task task-2 (Done)" in out
    assert "Running task task-3 (Done)" in out

    assert svc1.ran_once()
    assert tsk1.ran_once()
    assert tsk2.ran_once()
    assert tsk3.ran_once()

    # assert service ran before all three tasks
    assert svc1.last_ran > tsk1.last_ran
    assert svc1.last_ran > tsk2.last_ran
    assert svc1.last_ran > tsk3.last_ran


def test_deploy_ordered_dependencies_in_order(cartel):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        ordered_dependencies: [task-1, task-2, task-3]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        ---
        kind: Task
        name: task-3
        shell: {tsk3.shell}
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "svc-1"])

    # THEN
    assert svc1.last_ran > tsk1.last_ran
    assert svc1.last_ran > tsk2.last_ran
    assert svc1.last_ran > tsk3.last_ran
    # assert ordered dependencies run in order
    assert tsk3.last_ran > tsk1.last_ran
    assert tsk3.last_ran > tsk2.last_ran
    assert tsk2.last_ran > tsk1.last_ran


def test_group_deploys_all_members(cartel):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        dependencies: [task-1, task-2, task-3]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        ---
        kind: Task
        name: task-3
        shell: {tsk3.shell}
        ---
        kind: Group
        name: group-1
        dependencies: [svc-1, task-1, task-2, task-3]
        """
    )

    # WHEN
    out = cartel.client_cmd(["deploy", "group-1"])

    # THEN
    assert "Deploying svc-1 (Deployed)" in out
    assert "Group group-1 (Done)" in out
    assert "Running task task-1 (Done)" in out
    assert "Running task task-2 (Done)" in out
    assert "Running task task-3 (Done)" in out

    assert svc1.ran_once()
    assert tsk1.ran_once()
    assert tsk2.ran_once()
    assert tsk3.ran_once()

    # assert service ran before all three tasks
    assert svc1.last_ran > tsk1.last_ran
    assert svc1.last_ran > tsk2.last_ran
    assert svc1.last_ran > tsk3.last_ran


def test_deploys_multiple_services_and_groups(cartel):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    svc3 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        dependencies: [task-1, task-2]
        ---
        kind: Service
        name: svc-2
        shell: {svc2.shell}
        dependencies: [task-2]
        ---
        kind: Service
        name: svc-3
        shell: {svc3.shell}
        dependencies: [task-1]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        ---
        kind: Task
        name: task-3
        shell: {tsk3.shell}
        ---
        kind: Group
        name: group-1
        dependencies: [svc-3, task-1, task-2]
        ---
        kind: Group
        name: group-2
        dependencies: [svc-2, svc-3]
        """
    )

    # WHEN
    out = cartel.client_cmd(
        ["deploy", "group-1", "group-2", "svc-1", "svc-2", "svc-3"],
    )

    # THEN
    assert "Deploying svc-1 (Deployed)" in out
    assert "Deploying svc-2 (Deployed)" in out
    assert "Deploying svc-3 (Deployed)" in out
    assert "Group group-1 (Done)" in out
    assert "Group group-2 (Done)" in out
    assert "Running task task-1 (Done)" in out
    assert "Running task task-2 (Done)" in out

    assert svc1.ran_once()
    assert svc2.ran_once()
    assert svc3.ran_once()
    assert tsk1.ran_once()
    assert tsk2.ran_once()
    assert not tsk3.ran()


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

    # # WHEN/THEN
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


def test_deploy_skips_already_deployed_services_and_tasks(cartel):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    svc3 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        dependencies: [task-1, task-2]
        ---
        kind: Service
        name: svc-2
        shell: {svc2.shell}
        dependencies: [task-3]
        ---
        kind: Service
        name: svc-3
        shell: {svc3.shell}
        dependencies: [task-1]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        ---
        kind: Task
        name: task-3
        shell: {tsk3.shell}
        ---
        kind: Group
        name: group-1
        dependencies: [svc-1]
        ---
        kind: Group
        name: group-2
        dependencies: [svc-1, svc-2, svc-3]
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "group-1"])
    out = cartel.client_cmd(["deploy", "group-2"])

    # THEN
    assert "Deploying svc-1 (Already deployed)" in out
    assert "Deploying svc-2 (Deployed)" in out
    assert "Deploying svc-3 (Deployed)" in out
    assert "Running task task-1 (Done)" in out
    assert "Running task task-2 (Skipping)" in out
    assert "Running task task-3 (Done)" in out
    assert "Group group-2 (Done)" in out


def test_deploy_does_not_skip_already_deployed_services_and_tasks_if_f_flag_is_on(
    cartel,
):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    svc3 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        dependencies: [task-1, task-2]
        ---
        kind: Service
        name: svc-2
        shell: {svc2.shell}
        dependencies: [task-3]
        ---
        kind: Service
        name: svc-3
        shell: {svc3.shell}
        dependencies: [task-1]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        ---
        kind: Task
        name: task-3
        shell: {tsk3.shell}
        ---
        kind: Group
        name: group-1
        dependencies: [svc-1]
        ---
        kind: Group
        name: group-2
        dependencies: [svc-1, svc-2, svc-3]
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "group-1"])
    out = cartel.client_cmd(["deploy", "-f", "group-2"])

    # THEN
    assert "Deploying svc-1 (Deployed)" in out
    assert "Deploying svc-2 (Deployed)" in out
    assert "Deploying svc-3 (Deployed)" in out
    assert "Running task task-1 (Done)" in out
    assert "Running task task-2 (Done)" in out
    assert "Running task task-3 (Done)" in out
    assert "Group group-2 (Done)" in out


def test_deploy_skips_dependencies_with_only_selected(cartel):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        dependencies: [task-1, task-2, task-3]
        ---
        kind: Service
        name: svc-2
        shell: {svc2.shell}
        dependencies: [task-1, task-2, task-3]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        ---
        kind: Task
        name: task-3
        shell: {tsk3.shell}
        """
    )

    # WHEN
    out = cartel.client_cmd(["deploy", "--only-selected", "svc-1", "svc-2"])

    # THEN
    assert "Deploying svc-1 (Deployed)" in out
    assert "Deploying svc-2 (Deployed)" in out
    assert "Running task task-1 (Done)" not in out
    assert "Running task task-2 (Done)" not in out
    assert "Running task task-3 (Done)" not in out

    assert svc1.ran_once()
    assert svc2.ran_once()
    assert not tsk1.ran()
    assert not tsk2.ran()
    assert not tsk3.ran()
