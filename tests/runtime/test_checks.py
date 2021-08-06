from runtime.shim import check_shim, service_shim


def test_checks_run_for_single_service(cartel):
    # GIVEN
    svc = service_shim()
    check1 = check_shim()
    check2 = check_shim()
    check3 = check_shim()

    cartel.definitions(
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
    out = cartel.client_cmd(["deploy", "my-module"])

    # THEN
    assert "Check about check-1 (check-1) (OK)" in out
    assert "Check about check-2 (check-2) (OK)" in out
    assert "Check about check-3 (check-3) (OK)" not in out
    assert check1.ran_once()
    assert check2.ran_once()
    assert not check3.ran()


def test_check_failing_check_will_prevent_deploy(cartel):
    # GIVEN
    svc = service_shim()
    check1 = check_shim()
    check2 = check_shim(exit_code=1)
    check3 = check_shim()

    cartel.definitions(
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
    out = cartel.client_cmd(["deploy", "my-module"])

    # THEN
    assert check1.ran_once()
    assert check2.ran_once()
    assert not check3.ran()

    assert "The about check-2 check has failed" in out
    assert "Message: help check-2" in out


def test_check_only_run_once(cartel):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    check1 = check_shim()
    check2 = check_shim()

    cartel.definitions(
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
    cartel.client_cmd(["deploy", "svc1", "svc2"])

    # THEN
    assert check1.ran_once()
    assert check2.ran_once()


def test_checks_dont_run_with_no_checks_flag(cartel):
    # GIVEN
    svc1 = service_shim()
    svc2 = service_shim()
    check1 = check_shim()
    check2 = check_shim()

    cartel.definitions(
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
    out = cartel.client_cmd(["deploy", "--no-checks", "svc1", "svc2"])

    # THEN
    assert "Check about check-1 (check-1) (OK)" not in out
    assert "Check about check-2 (check-2) (OK)" not in out
    assert not check1.ran()
    assert not check2.ran()
