from runtime.client import client_cmd_tty
from runtime.helpers import definition, run_service
from runtime.shim import task_shim


def test_prints_logs_for_service(daemon):
    # GIVEN
    run_service("logs-1")

    # WHEN/THEN
    with client_cmd_tty(["logs", "logs-1"]) as tty:
        assert tty.expect(pattern="pass")


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
    with client_cmd_tty(["logs", "logs-2"], defs=definitions_file) as tty:
        assert tty.expect(pattern="pass")
