from runtime.shim import env_shim


def test_environment_set_activates(cartel):
    # GIVEN
    svc = env_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        environment:
            var1: "var1-base"
            var2: "var2-base"
        environment_sets:
            debug:
                var1: "var1-debug"
                var3: "var3-debug"
        """
    )

    # WHEN
    cartel.client_cmd(["deploy", "-e", "debug", "svc"])

    # THEN
    assert "var1" in svc.environment_vars
    assert "var2" in svc.environment_vars
    assert "var3" in svc.environment_vars
    assert svc.environment_vars["var1"] == "var1-debug"
    assert svc.environment_vars["var2"] == "var2-base"
    assert svc.environment_vars["var3"] == "var3-debug"


def test_multiple_environment_sets_priority(cartel):
    # GIVEN
    svc = env_shim()

    cartel.definitions(
        f"""
        kind: Service
        name: svc
        shell: {svc.shell}
        environment:
            var1: "var1-base"
            var2: "var2-base"
        environment_sets:
            debug:
                var2: "var2-debug"
                var3: "var3-debug"
                var4: "var4-debug"
                var5: "var5-debug"
            staging:
                var3: "var3-staging"
                var4: "var4-staging"
                var6: "var6-staging"
            prod:
                var4: "var4-prod"
                var7: "var7-prod"
        """
    )

    # WHEN
    cartel.client_cmd(
        ["deploy", "-e", "debug", "-e", "staging", "-e", "prod", "svc"]
    )

    # THEN
    assert "var1" in svc.environment_vars
    assert "var2" in svc.environment_vars
    assert "var3" in svc.environment_vars
    assert svc.environment_vars["var1"] == "var1-base"
    assert svc.environment_vars["var2"] == "var2-debug"
    assert svc.environment_vars["var3"] == "var3-staging"
    assert svc.environment_vars["var4"] == "var4-prod"
    assert svc.environment_vars["var5"] == "var5-debug"
    assert svc.environment_vars["var6"] == "var6-staging"
    assert svc.environment_vars["var7"] == "var7-prod"
