# Testing status

Scenarios and fields covered: ✅

## Scenarios
- ~~Perform `deploy` for single service ✅~~
- ~~Perform `deploy` for multiple services and tasks ✅~~
- ~~Perform `deploy` for group ✅~~
- ~~Perform `stop` for single service ✅~~
- ~~Perform `stop` multiple services ✅~~
- ~~Perform `restart` service ✅~~
- Flag `--only-selected` for `deploy`
- Flag `--serial` for `deploy`
- Flag `--no-checks` for `deploy`
- Flag `--no-readiness` for `deploy`
- Flag `--env` for `deploy`
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
- log_file_path (untested)
- working_dir (untested)
- ordered_dependencies (untested)
- readiness_probe (untested)
- liveness_probe (untested)
- post_up (untested)
- post (untested)
- after (untested)

## Task
- ~~name (tested) ✅~~
- ~~checks (tested) ✅~~
- ~~shell (tested) ✅~~
- environment (untested)
- working_dir (untested)
- timeout (untested)

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
- host (untested)
- port (untested)
- retries (untested)

### Exec
- command (untested)
- shell (untested)
- working_dir (untested)
- retries (untested)

### Log line
- line_regex (untested)
- retries (untested)
