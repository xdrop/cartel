from runtime.helpers import run_service, run_task
from runtime.shim import task_shim


def test_prints_logs_for_service(cartel):
    # GIVEN
    run_service("logs-1")

    # WHEN/THEN
    with cartel.client_cmd_tty(["logs", "logs-1"]) as tty:
        assert tty.expect(pattern="pass")


def test_prints_logs_for_task(cartel):
    # GIVEN
    tsk = task_shim()

    cartel.definitions(
        f"""
        kind: Task
        name: logs-2
        shell: {tsk.shell}
        """
    )
    run_task("logs-2")

    # WHEN/THEN
    with cartel.client_cmd_tty(["logs", "logs-2"]) as tty:
        assert tty.expect(pattern="pass")
