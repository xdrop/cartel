import re

from runtime.client import client_cmd
from runtime.helpers import run_service, stop_service


def test_ps_with_nothing_running(daemon):
    # GIVEN
    out = client_cmd(["ps"])

    # THEN
    assert (
        out.replace("\n", "").replace("\r", "")
        == "pid       name      liveness  status    since"
    )


def test_ps_prints_service_running(daemon):
    # GIVEN
    run_service("ps-1")

    # WHEN
    out = client_cmd(["ps"]).splitlines()

    # THEN
    assert out[0] == "pid       name      liveness  status    since"
    # matches 17584     ps-1      -         running   now
    assert re.match(r"\d+\s+ps-1\s+-\s+running\s+.*", out[1])


def test_ps_prints_multiple_services_running(daemon):
    # GIVEN
    run_service("ps-1")
    run_service("ps-2")
    run_service("ps-3")

    # WHEN
    out = client_cmd(["ps"])

    # THEN
    assert re.match(r"pid       name      liveness  status    since", out)
    assert re.findall(r"^\d+\s+ps-1\s+-\s+running\s+.*", out, re.M)
    assert re.findall(r"^\d+\s+ps-2\s+-\s+running\s+.*", out, re.M)
    assert re.findall(r"^\d+\s+ps-3\s+-\s+running\s+.*", out, re.M)


def test_ps_prints_correct_run_status_for_multiple_services(daemon):
    # GIVEN
    run_service("ps-1")
    run_service("ps-2")
    stop_service("ps-2")
    run_service("ps-3", exit_code=1)

    # WHEN
    out = client_cmd(["ps"])

    # THEN
    assert re.match(r"pid       name      liveness  status    since", out)
    assert re.findall(r"^\d+\s+ps-1\s+-\s+running\s+.*", out, re.M)
    assert re.findall(r"^\d+\s+ps-2\s+-\s+stopped\s+.*", out, re.M)
    assert re.findall(r"^\d+\s+ps-3\s+-\s+exited\s+.*", out, re.M)
