import tempfile

from runtime.shim import service_shim


def test_exec_cmd_for_service(cartel):
    # GIVEN
    svc = service_shim()
    tmp_dir = tempfile.mkdtemp()

    cartel.definitions(
        f"""
        kind: Service
        name: exec-test
        shell: {svc.shell}
        working_dir: {tmp_dir}
        """
    )

    # WHEN
    out = cartel.client_cmd(["exec", "exec-test", "pwd"])

    # THEN
    # not doing an exact match due to /var symlinked to /private/var on macOS
    assert tmp_dir in out
