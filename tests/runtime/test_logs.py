from runtime.helpers import client_cmd_tty_expect, definition, run_service
from runtime.shim import task_shim


def test_prints_logs_for_service(daemon):
    # GIVEN
    run_service("logs-1")

    # WHEN/THEN
    assert client_cmd_tty_expect(["logs", "logs-1"], pattern="pass")


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

    # WHEN/THEN
    assert client_cmd_tty_expect(
        ["logs", "logs-2"], pattern="pass", defs=definitions_file
    )
