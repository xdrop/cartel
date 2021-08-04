from runtime.helpers import client_cmd, definition, run_service
from runtime.shim import task_shim


def test_prints_logs_for_service(daemon):
    # GIVEN
    run_service("logs-1")

    # WHEN
    out = client_cmd(["logs", "logs-1"], blocking=True)

    # THEN
    assert out == "pass\n"


def test_prints_logs_for_task(daemon):
    # GIVEN
    tsk = task_shim()

    definitions_file = definition(
        f"""
        kind: Task
        name: logs-2
        shell: {tsk.shell}
        """
    )

    # WHEN
    out = client_cmd(["logs", "logs-2"], blocking=True, defs=definitions_file)

    # THEN
    assert out == "pass\n"
