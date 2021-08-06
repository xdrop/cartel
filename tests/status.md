# Testing status

Scenarios and fields covered: ✅

## Scenarios
- ~~Perform `deploy` for single service ✅~~
- ~~Perform `deploy` for multiple services and tasks ✅~~
- ~~Perform `deploy` for group ✅~~
- ~~Perform `stop` for single service ✅~~
- ~~Perform `stop` multiple services ✅~~
- ~~Perform `restart` service ✅~~
- ~~Deploy won't redeploy healthy services ✅~~
- ~~Deploy won't redeploy tasks if all their services are healthy ✅~~
- ~~Deploy will always redeploy if `--force` is on ✅~~
- ~~Flag `--no-checks` for `deploy` ✅~~
- ~~Flag `--only-selected` for `deploy` ✅~~
- Flag `--no-readiness` for `deploy`
- Flag `--env` for `deploy`
- Flag `--serial` for `deploy`
- Perform `run` for single task running
- ~~View `logs` (standard mode) ✅~~
- View `logs` (full mode)
- Start `shell` for service
- ~~View `ps` status ✅~~
- Perform `down` for stopping all services
- Perform `config get` for getting config
- Perform `config set` for setting config
- Perform `config removing` for removing config
- Perform `config toggle` for inverting config
- Perform `config view` for printing config

## Service
- ~~name (tested) ✅~~
- ~~shell (tested) ✅~~
- ~~checks (tested) ✅~~
- ~~dependencies (tested) ✅~~
- ~~termination_signal (tested) ✅~~
- ~~command (tested) ✅~~
- ~~environment (tested) ✅~~
- environment_sets (untested)
- ~~log_file_path (tested) ✅~~
- ~~working_dir (tested) ✅~~
- ~~ordered_dependencies (tested) ✅~~
- ~~readiness_probe (tested) ✅~~
- ~~liveness_probe (tested) ✅~~
- post_up (untested)
- post (untested)
- after (untested)

## Task
- ~~name (tested) ✅~~
- ~~checks (tested) ✅~~
- ~~shell (tested) ✅~~
- ~~command (tested) ✅~~
- ~~working_dir (tested) ✅~~
- ~~environment (tested) ✅~~
- ~~log_file_path (tested) ✅~~
- ~~timeout (tested) ✅~~

## Shell
- name (untested)
- service (untested)
- command (untested)
- shell (untested)
- shell_type (untested)
- environment (untested)
- working_dir (untested)

## Group
- ~~name: (tested)   ✅~~
- ~~dependencies (tested) ✅~~
- ~~checks: (tested) ✅~~

## Check
- ~~name (tested) ✅~~
- ~~about (tested) ✅~~
- ~~shell (tested) ✅~~
- ~~help (tested) ✅~~
- suggested_fix (untested)
- working_dir (untested)

## Probe

### Net
- ~~host (tested) ✅~~
- ~~port (tested) ✅~~
- ~~retries (tested) ✅~~

### Exec
- ~~command (tested) ✅~~
- ~~shell (tested) ✅~~
- ~~working_dir (tested) ✅~~
- ~~retries (tested) ✅~~

### Log line
- ~~line_regex (tested) ✅~~
- ~~retries (tested) ✅~~
