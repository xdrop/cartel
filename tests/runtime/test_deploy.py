from runtime.helpers import client_cmd, definition
from runtime.shim import service_shim, task_shim


def test_deploy_single_service(daemon):
    # GIVEN
    svc = service_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        """
    )

    # WHEN
    out = client_cmd(["deploy", "svc"], defs=definitions_file)

    # THEN
    assert "Deploying svc (Deployed)" in out
    assert 'Deployed modules: ["svc"]' in out
    assert svc.ran_once()


def test_deploy_tasks_before_service(daemon):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    definitions_file = definition(
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
    out = client_cmd(["deploy", "svc-1"], defs=definitions_file)

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


def test_group_deploys_all_members(daemon):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    definitions_file = definition(
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
    out = client_cmd(["deploy", "group-1"], defs=definitions_file)

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


def test_deploys_multiple_services_and_groups(daemon):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    svc3 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    definitions_file = definition(
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
    out = client_cmd(
        ["deploy", "group-1", "group-2", "svc-1", "svc-2", "svc-3"],
        defs=definitions_file,
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
