from runtime.client import client_cmd
from runtime.helpers import definition
from runtime.shim import env_shim, log_file_shim, task_shim, working_dir_shim


def test_command_works_for_task(daemon):
    # GIVEN
    tsk = task_shim()

    definitions_file = definition(
        f"""
        kind: Task
        name: tsk
        command: {tsk.command}
        """
    )

    # WHEN
    client_cmd(["deploy", "tsk"], defs=definitions_file)

    # THEN
    assert tsk.ran_once()


def test_environment_variables_get_set_for_task(daemon):
    # GIVEN
    tsk = env_shim()

    definitions_file = definition(
        f"""
        kind: Task
        name: tsk
        shell: {tsk.shell}
        environment:
            var1: "foo"
            var2: "bar"
        """
    )

    # WHEN
    client_cmd(["deploy", "tsk"], defs=definitions_file)

    # THEN
    assert "var1" in tsk.environment_vars
    assert "var2" in tsk.environment_vars
    assert tsk.environment_vars["var1"] == "foo"
    assert tsk.environment_vars["var2"] == "bar"


def test_task_deployed_in_working_dir(daemon):
    # GIVEN
    tsk = working_dir_shim()

    definitions_file = definition(
        f"""
        kind: Task
        name: tsk
        shell: {tsk.shell}
        working_dir: {tsk.working_dir}
        """
    )

    # WHEN
    client_cmd(["deploy", "tsk"], defs=definitions_file)

    # THEN
    assert tsk.ran_in_workdir


def test_logs_are_written_to_given_file(daemon):
    # GIVEN
    tsk = log_file_shim()

    definitions_file = definition(
        f"""
        kind: Task
        name: tsk
        shell: {tsk.shell}
        log_file_path: {tsk.log_file_path}
        """
    )

    # WHEN
    client_cmd(["deploy", "tsk"], defs=definitions_file)

    # THEN
    assert tsk.written_to_log_file


def test_task_timeout_exceed(daemon):
    # GIVEN
    tsk = task_shim(delay=5)

    print(tsk.shell)
    definitions_file = definition(
        f"""
        kind: Task
        name: tsk
        shell: {tsk.shell}
        timeout: 1
        """
    )

    # WHEN
    out = client_cmd(["deploy", "tsk"], timeout=3, defs=definitions_file)

    # THEN
    assert 'Error: Task "tsk" took too long to finish.' in out
    assert (
        "Try increasing the timeout"
        " or check the logs using `cartel logs tsk`." in out
    )
    assert "Note: The task may still be running." in out


def test_task_timeout_not_exceed(daemon):
    # GIVEN
    tsk = task_shim(delay=1)

    definitions_file = definition(
        f"""
        kind: Task
        name: tsk
        shell: {tsk.shell}
        timeout: 3
        """
    )

    # WHEN
    out = client_cmd(["deploy", "tsk"], timeout=2, defs=definitions_file)

    # THEN
    assert "Running task tsk (Done)" in out
