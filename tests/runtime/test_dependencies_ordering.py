import pytest

from runtime.shim import exit_toggle_shim, service_shim, task_shim


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


def test_after_enforces_dependency_ordering(cartel):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        dependencies: [task-1]
        ---
        kind: Service
        name: svc-2
        shell: {svc2.shell}
        dependencies: [task-2]
        ---
        kind: Task
        name: task-1
        after: [task-2]
        shell: {tsk1.shell}
        ---
        kind: Task
        name: task-2
        shell: {tsk2.shell}
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "svc-1", "svc-2"])

    # THEN
    assert svc1.last_ran > tsk1.last_ran
    # assert tsk1 after tsk2
    assert tsk1.last_ran > tsk2.last_ran


def test_post_runs_post_deployment(cartel):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        post: [task-1]
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "svc-1"])

    # THEN
    assert svc1.last_ran < tsk1.last_ran


@pytest.mark.slow
def test_post_up_runs_post_readiness(cartel):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    probe = exit_toggle_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc-1
        shell: {svc1.shell}
        post: [task-1]
        readiness_probe:
            type: exec
            shell: {probe.shell}
            retries: 10
        ---
        kind: Task
        name: task-1
        shell: {tsk1.shell}
        """
    )

    # WHEN/THEN
    with cartel.client_cmd_tty(["deploy", "svc-1"]) as tty:
        assert tty.expect(pattern="Waiting", timeout=1)
        # assert the task has not run yet
        assert not tsk1.ran()
        # make the readiness probe pass
        probe.toggle()
        # wait for EOF
        tty.expect(timeout=10)
        # assert task has now run
        assert tsk1.ran(force_update=True)
