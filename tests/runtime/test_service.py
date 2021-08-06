import pytest

from runtime.shim import env_shim, log_file_shim, service_shim, working_dir_shim


@pytest.mark.parametrize("cmd_line_type", [("shell"), ("command")])
def test_cmd_works_for_service(cmd_line_type, cartel):
    # GIVEN
    svc = service_shim()

    cmd_line = getattr(svc, cmd_line_type)
    cartel.definitions(
        f"""
        kind: Service
        name: svc
        {cmd_line_type}: {cmd_line}
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "svc"])

    # THEN
    assert svc.ran_once()


def test_environment_variables_get_set(cartel):
    # GIVEN
    svc = env_shim()

    cartel.definitions(
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
    cartel.client_cmd(["deploy", "svc"])

    # THEN
    assert "var1" in svc.environment_vars
    assert "var2" in svc.environment_vars
    assert svc.environment_vars["var1"] == "foo"
    assert svc.environment_vars["var2"] == "bar"


def test_logs_are_written_to_given_file(cartel):
    # GIVEN
    svc = log_file_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        log_file_path: {svc.log_file_path}
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "svc"])

    # THEN
    assert svc.written_to_log_file


def test_deployed_in_working_dir(cartel):
    # GIVEN
    svc = working_dir_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        working_dir: {svc.working_dir}
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "svc"])

    # THEN
    assert svc.ran_in_workdir
