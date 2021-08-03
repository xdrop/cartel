from runtime.helpers import client_cmd, definition
from runtime.shim import service_shim, task_shim


def test_deploy_single_service(daemon):
    # GIVEN
    svc = service_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: my-module
        shell: {svc.shell}
        """
    )

    # WHEN
    out = client_cmd(["deploy", "my-module"], defs=definitions_file)

    # THEN
    assert 'Deployed modules: ["my-module"]' in out
    assert svc.ran()


def test_deploy_tasks_before_service(daemon):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: my-module
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
    client_cmd(["deploy", "my-module"], defs=definitions_file)

    # THEN
    svc1_time = svc1.ran()
    assert svc1_time
    tsk1_time = tsk1.ran()
    assert tsk1_time
    tsk2_time = tsk2.ran()
    assert tsk2_time
    tsk3_time = tsk3.ran()
    assert tsk3_time

    # assert service ran before all three tasks
    assert svc1_time > tsk1_time
    assert svc1_time > tsk2_time
    assert svc1_time > tsk3_time


def test_group_deploys_all_members(daemon):
    # GIVEN
    svc1 = service_shim()
    tsk1 = task_shim()
    tsk2 = task_shim()
    tsk3 = task_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: my-module
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
        dependencies: [my-module, task-1, task-2, task-3]
        """
    )

    # WHEN
    client_cmd(["deploy", "group-1"], defs=definitions_file)

    # THEN
    svc1_time = svc1.ran()
    assert svc1_time
    tsk1_time = tsk1.ran()
    assert tsk1_time
    tsk2_time = tsk2.ran()
    assert tsk2_time
    tsk3_time = tsk3.ran()
    assert tsk3_time

    # assert service ran before all three tasks
    assert svc1_time > tsk1_time
    assert svc1_time > tsk2_time
    assert svc1_time > tsk3_time


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
    client_cmd(
        ["deploy", "group-1", "group-2", "svc-1", "svc-2", "svc-3"],
        defs=definitions_file,
    )

    # THEN
    assert svc1.ran()
    assert svc2.ran()
    assert svc3.ran()
    assert tsk1.ran()
    assert tsk2.ran()
    assert not tsk3.ran()
