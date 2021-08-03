from runtime.helpers import client_cmd, definition
from runtime.shim import check_shim, service_shim


def test_checks_run_for_single_service(daemon):
    # GIVEN
    svc = service_shim()
    check1 = check_shim()
    check2 = check_shim()
    check3 = check_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: my-module
        shell: {svc.shell}
        checks: [check-1, check-2]
        ---
        kind: Check
        name: check-1
        shell: {check1.shell}
        help: help check-1
        about: about check-1
        ---
        kind: Check
        name: check-2
        shell: {check2.shell}
        help: help check-2
        about: about check-2
        ---
        kind: Check
        name: check-3
        shell: {check3.shell}
        help: help check-3
        about: about check-3
        """
    )

    # WHEN
    out = client_cmd(["deploy", "my-module"], defs=definitions_file)

    # THEN
    assert check1.ran()
    assert check2.ran()
    assert not check3.ran()


def test_check_failing_check_will_prevent_deploy(daemon):
    # GIVEN
    svc = service_shim()
    check1 = check_shim()
    check2 = check_shim(exit_code=1)
    check3 = check_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: my-module
        shell: {svc.shell}
        checks: [check-1, check-2]
        ---
        kind: Check
        name: check-1
        shell: {check1.shell}
        help: help check-1
        about: about check-1
        ---
        kind: Check
        name: check-2
        shell: {check2.shell}
        help: help check-2
        about: about check-2
        ---
        kind: Check
        name: check-3
        shell: {check3.shell}
        help: help check-3
        about: about check-3
        """
    )

    # WHEN
    out = client_cmd(["deploy", "my-module"], defs=definitions_file)

    # THEN
    assert check1.ran()
    assert check2.ran()
    assert not check3.ran()

    assert "The about check-2 check has failed" in out
    assert "Message: help check-2" in out


def test_check_only_run_once(daemon):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    check1 = check_shim()
    check2 = check_shim()

    definitions_file = definition(
        f"""
        kind: Service
        name: svc1
        shell: {svc1.shell}
        checks: [check-1, check-2]
        ---
        kind: Service
        name: svc2
        shell: {svc2.shell}
        checks: [check-2]
        ---
        kind: Check
        name: check-1
        shell: {check1.shell}
        help: help check-1
        about: about check-1
        ---
        kind: Check
        name: check-2
        shell: {check2.shell}
        help: help check-2
        about: about check-2
        """
    )

    # WHEN
    client_cmd(["deploy", "svc1", "svc2"], defs=definitions_file)

    # THEN
    # implied: This fails if shim is executed more than once
    assert check1.ran()
    assert check2.ran()
