import pytest

from runtime.client import client_cmd
from runtime.helpers import definition
from runtime.shim import env_shim, log_file_shim, service_shim, working_dir_shim


@pytest.mark.parametrize("cmd_line_type", [("shell"), ("command")])
def test_cmd_works_for_service(cmd_line_type, daemon):
    # GIVEN
    svc = service_shim()

    cmd_line = getattr(svc, cmd_line_type)
    definitions_file = definition(
        f"""
        kind: Service
        name: svc
        {cmd_line_type}: {cmd_line}
        """
    )

    # WHEN
    client_cmd(["deploy", "svc"], defs=definitions_file)

    # THEN
    assert svc.ran_once()


def test_environment_variables_get_set(daemon):
    # GIVEN
    svc = env_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        environment:
            var1: "foo"
            var2: "bar"
        """
    )

    # WHEN
    client_cmd(["deploy", "svc"], defs=definitions_file)

    # THEN
    assert "var1" in svc.environment_vars
    assert "var2" in svc.environment_vars
    assert svc.environment_vars["var1"] == "foo"
    assert svc.environment_vars["var2"] == "bar"


def test_logs_are_written_to_given_file(daemon):
    # GIVEN
    svc = log_file_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        log_file_path: {svc.log_file_path}
        """
    )

    # WHEN
    client_cmd(["deploy", "svc"], defs=definitions_file)

    # THEN
    assert svc.written_to_log_file


def test_deployed_in_working_dir(daemon):
    # GIVEN
    svc = working_dir_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        working_dir: {svc.working_dir}
        """
    )

    # WHEN
    client_cmd(["deploy", "svc"], defs=definitions_file)

    # THEN
    assert svc.ran_in_workdir
