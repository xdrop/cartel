# cartel

> Local development workflow orchestrator

`cartel` is an orchestration tool aimed at making local development easier for complex systems with multiple services. It allows for codifying the steps required to run some system locally, and providing an easy consistent interface for managing that system. It was heavily inspired by `docker-compose`, `k8s`, and `garden`, but instead works without containers, with both the benefits and drawbacks this approach entails.

[![screencap](./img/screencap.png)](https://asciinema.org/a/PncJI8AS795r9zwXWLViXlTAb)

## Table of contents
- [cartel](#cartel)
  - [Table of contents](#table-of-contents)
  - [Features](#features)
    - [Deploying services](#deploying-services)
    - [Viewing logs](#viewing-logs)
    - [Running tasks](#running-tasks)
    - [Viewing service status](#viewing-service-status)
    - [Stopping / restarting a service](#stopping--restarting-a-service)
    - [Opening a REPL shell](#opening-a-repl-shell)
  - [Getting started configuration](#getting-started-configuration)
  - [Installation](#installation)
      - [macOS](#macos)
      - [Linux](#linux)
      - [Windows](#windows)
  - [Reference manual](#reference-manual)
    - [Service definition](#service-definition)
      - [Example](#example)
    - [Task definition](#task-definition)
      - [Example](#example-1)
    - [Shell definition](#shell-definition)
      - [Example](#example-2)
    - [Group definition](#group-definition)
      - [Example](#example-3)
    - [Check definition](#check-definition)
      - [Example](#example-4)
    - [Environment sets](#environment-sets)
    - [Readiness and Liveness probes](#readiness-and-liveness-probes)
      - [Net probe](#net-probe)
      - [Executable probe](#executable-probe)
      - [Log line probe](#log-line-probe)
    - [Suggested fix for checks](#suggested-fix-for-checks)
      

## Features
While `cartel` is still in development (see [CHANGELOG](../CHANGELOG.md)) it has a feature set rich enough to cover most use-cases.

Some of the features included are:

- Deploy services and orchestrate dependencies / tasks that need to be performed before or after a service is deployed.
- Tail and manage logs of services.
- Monitor the health of services.
- Running tasks ad-hoc.
- Convienient access to REPL shells for services.
- Perform checks to ensure a machine is set up in a correct state.

`cartel` revolves around modules which are used to codify your local services setup. There are currently _five_ different kinds of modules which are:
- **Task** - A task is a module with a limited lifetime, used to perform some temporary operation or some setup.
- **Service** - A service is a longer running module. Its lifetime will be managed and can be started, stopped independently.
- **Group** - A group is a module which serves as a grouping of other modules that need to be deployed together.
- **Shell** - A shell is a module which allows for a quick way to open a REPL shell for some service.
- **Check** - A check is a module which defines some condition which must evaluate to true before some service can be operated. This is used to give hints/solutions for common issues that occur during setting up a machine for local development for the first time. 




### Deploying services
To deploy a service (or group):
```
$ cartel deploy -f <name>
```

To deploy more than one service (or group):
```
$ cartel deploy -f <one> <two> <three> ...
```

The `-f` flag always forces deployment of all modules/tasks. If you don't want services in the correct state to be redeployed you can omit it.

### Viewing logs
To tail the logs of a service/task:

```
$ cartel logs <name>
```

To view *all* the logs of a service/task:

```
$ cartel logs -a <name>
```

### Running tasks
To run an ad-hoc task:

```
$ cartel run <task-name>
```

### Viewing service status
To view services and their status:

```
$ cartel ps
```

### Stopping / restarting a service
To start / stop a service:

```
$ cartel stop <name>
$ cartel restart <name>
```

### Opening a REPL shell
To open a REPL shell to some service. Since services can define multiple types of REPL shells `-t` can distinguish between them based on `type`.

```
$ cartel shell <service_name>
$ cartel shell -t <type> <service_name>
```

## Getting started configuration

Here is a sample configuration that defines one service (`backend`) and one task (`postgres:docker-up`) as a dependency of backend, along with one check (`backend:check-a`).
```
kind: Service
name: backend
checks: ["backend:check-a"]
shell: make local-run
working_dir: ./api/backend
dependencies: ["postgres:docker-up"]
environment:
    PORT: 8080
readiness_probe:
  type: net
  host: localhost
  port: 8080
liveness_probe:
  type: net
  host: localhost
  port: 8080
---
kind: Task
name: postgres:docker-up
shell: docker-compose up -d postgres
---
kind: Check
name: backend:check-a
about: some check
shell: echo "always succeed"
help: Instructions on how to fix
```

Run `cartel deploy -f backend` to try it out.

## Installation

#### macOS

```
brew tap xdrop/homebrew-tap
brew install cartel
```

and add the following to `~/.zshrc` and `~/.bashrc`:
```
[ -f /usr/local/opt/cartel/launch-daemon.sh ] && . /usr/local/opt/cartel/launch-daemon.sh
```

#### Linux
Linux users will need to compile manually (ensure `Rust Nightly` is installed).
```
$ git clone https://github.com/xdrop/cartel.git
$ cd cartel
$ cargo build --release --all
$ mv target/release/client /usr/local/bin/cartel
$ mv target/release/daemon /usr/local/bin/cartel-daemon
$ chmod +x /usr/local/bin/cartel*
$ mkdir -p ~/.cartel
$ cp launch-daemon.sh ~/.cartel
```

and add the following to `~/.zshrc` and `~/.bashrc`:

```
[ -f ~/.cartel/launch-daemon.sh ] && . ~/.cartel/launch-daemon.sh
```

#### Windows
Windows is not supported.

## Reference manual

### Service definition

Use `Service` for running long running processes that can be started, stopped and managed by the daemon.

| Property | Description | Values | Example |
| -------- | ----------- | ------ | ------- |
| kind | Type of the module. Use `Service` for services. | Service | `Service`
| name | The name of the service. Only **unique** names allowed. | String| `backend`
| command | A command with which to launch the service. This has to be an array of the path to the program and its arguments. This does not invoke a shell so things like pipes (`\|`) and other shell operators will not work unless explicitly run within a shell (eg. in `bash -c`). The `shell` option described below will always run the command in a shell and should be preferred if use of shell features is required. | String[] | `["bash", "-c", "echo hi"]`
| shell | A shell command with which to launch the service. Unlike `command` this is a cmd line string which is evaluated in a shell context (`bash`). Only **one of** `command`/`shell` must be present. | String | `echo "This support shell operations" > myfile`
| termination_signal | The termination signal to use when stopping the service (for UNIX based OS). Use `KILL` for `SIGKILL`, `TERM` for `SIGTERM`, and `INT` for `SIGINT`. (Optional) | KILL \| TERM \| INT | `"KILL"`
| environment | The environment variables to pass to the service. (Optional) | Map[String, String] | `HOST: localhost` <br/> `PORT: 8921`
| environment_sets | Sets of environment variables that can be toggled on or off. See example for more details. (Optional) | Map[String, Map[String, String]] | [Environment Sets](#environment-sets)
| log_file_path | Path to the log file where stdout and stderr is written. (Optional) | String | `/tmp/my_service.log`
| dependencies | A list of module names that have to be deployed _before_ this service runs. (Optional) | String[] | `["task-a", "service-a"]`
| ordered_dependencies | Same as `dependencies` but each dependency also depends on the previous one. For example in the case of `[a,b,c]` the dependencies are deployed in the following order: `a` then `b` then `c`. This guarantee is not provided by `dependencies`. Ordered dependencies can co-exist with dependencies. (Optional)| String[] | `["task-a", "service-a"]`
| after | A service or task that should always be deployed _after_ this service, but not a strict dependency of this service. (Optional) | String[] | `["task-a", "service-a"]`
| post | A list of tasks to perform after the service has been deployed. (Optional) | String[] | `["task-a", "task-b"]`
| post_up | A list of tasks to perform after the service has been deployed **and** had its readiness probe pass. (Optional) | String[] | `["task-a", "task-b"]`
| working_dir | The working directory all commands and paths are relative to. Relative directories are allowed and they are relative to the location of the `cartel.yml` file. (Optional) | String | `./services/my-service`
| checks | A list of checks to perform before the service is allowed to run. (Optional) | String[] | `["check-a", "check-b"]`
| readiness_probe | A probe to run with which to determine if the service is healthy. This is used when deploying to wait for the service to come up. (Optional) | Probe | [Readiness & Liveness Probes](#readiness-and-liveness-probes)
| liveness_probe | A probe to run with which to determine if the service is healthy. This is used **after** the service has been deployed to monitor its ongoing health status. This affects things like `cartel ps` and skipping deploying a module if it is already in the correct state and has a passing liveness probe. (Optional) | Probe | [Readiness & Liveness Probes](#readiness-and-liveness-probes)

#### Example
```
kind: Service
name: backend
shell: make run
checks: ["backend:image_is_built"]
environment:
  PYTHONUNBUFFERED: "1"
  VSCODE_DEBUG: "true"
environment_sets:
  prod:
    DJANGO_SETTINGS_MODULE: "settings.prod"
  debug:
    DJANGO_LOG_LEVEL: DEBUG
working_dir: ./api/backend
ordered_dependencies: ["backing-services", "backend:poetry_install", "backend:run_migrations"]
readiness_probe:
  type: net
  retries: 18
  host: localhost
  port: 5000
liveness_probe:
  type: net
  host: localhost
  port: 5000
post_up: ["backend:refresh_cache"]
```

### Task definition

Use `Task` for short lived processes used to perform some temporary operation or setup.

| Property | Description | Values | Example |
| -------- | ----------- | ------ | ------- |
| kind | Type of the module. Use `Task` for tasks. | Task | `Task`
| name | The name of the task. Only **unique** names allowed. | String| `backend:run-migrations`
| command | A command with which to launch the task. This has to be an array of the path to the program and its arguments. This does not invoke a shell so things like pipes (`\|`) and other shell operators will not work unless explicitly run within a shell (eg. in `bash -c`). The `shell` option described below will always run the command in a shell and should be preferred if use of shell features is required. | String[] | `["bash", "-c", "echo hi"]`
| shell | A shell command with which to launch the task. Unlike `command` this is a cmd line string which is evaluated in a shell context (`bash`). Only **one of** `command`/`shell` must be present. | String | `echo "This support shell operations" > myfile`
| environment | The environment variables to pass to the task. (Optional) | Map[String, String] | `HOST: localhost` <br/> `PORT: 8921`
| log_file_path | Path to the log file where stdout and stderr is written. (Optional) | String | `/tmp/my_service.log`
| working_dir | The working directory all commands and paths are relative to.  Relative directories are allowed and they are relative to the location of the `cartel.yml` file. (Optional) | String | `./services/my-service`
| timeout | Number of seconds without completion before the task is considered failed. If left unspecified this will default to `180` seconds. (Optional) | u64 | 180

#### Example

```
kind: Task
name: backend:run_migrations
environment:
  PYTHONUNBUFFERED: "1"
shell: poetry run python manage.py migrate
working_dir: ./api/backend
```

### Shell definition

Use `Shell` to define a shortcut for getting a REPL shell for some service.

| Property | Description | Values | Example |
| -------- | ----------- | ------ | ------- |
| kind | Type of the module. Use `Shell` for shells. | `Shell` | `Shell`
| name | The name of the shell. Only **unique** names allowed. | String| `backend:shell`
| service | The service this shell is for. This has to match the module name of a service and is **required**. It is what `cartel shell` uses to determine which shell to open. | String | `myservicename`
| command | A command with which to launch the shell. This has to be an array of the path to the program and its arguments. This does not invoke a shell so things like pipes (`\|`) and other shell operators will not work unless explicitly run within a shell (eg. in `bash -c`). | String[] | `["bash", "-c", "echo hi"]`
| shell | A shell command with which to launch the shell. Unlike `command` this is a cmd line string which is evaluated in a shell context (`bash`). Only **one of** `command`/`shell` must be present. | String | `python3 $(get-shell)`
| shell_type | The type of the shell. Used to choose between multiple shell options for a service when specifying the `-t` option (eg. `cartel shell -t ipython myservice`) | String | `ipython`
| environment | The environment variables to pass to the shell. (Optional) | Map[String, String] | `HOST: localhost` <br/> `PORT: 8921`
| working_dir | The working directory all commands and paths are relative to. Relative directories are allowed and they are relative to the location of the `cartel.yml` file. (Optional) | String | `./services/my-service`


#### Example

```
kind: Shell
name: backend:shell
service: backend
command: [poetry, run, ipython]
working_dir: ./api/backend
```

### Group definition

Use `Group` for groupping sets of dependencies that need to be deployed together.

| Property | Description | Values | Example |
| -------- | ----------- | ------ | ------- |
| kind | Type of the module. Use `Group` for groups. | Group | `Group`
| name | The name of the group. Only **unique** names allowed. | String| `groupname`
| dependencies | A list of module names that consist this group. When the group is deployed all these dependencies are deployed. | String[] | `["task-a", "service-a"]`
| checks | A list of checks to perform before the group is allowed to run. (Optional) | String[] | `["check-a", "check-b"]`

#### Example

```
kind: Group
name: backing-services
checks: ["backing_services:hosts_file_entries"]
dependencies:
 - "kafka:docker_up"
 - "postgres:docker_up"
 - "nginx:docker_up"
```

### Check definition

Use `Check` to enforce a condition before a service is run (eg. to ensure some local configuration has been performed on the system).

| Property | Description | Values | Example |
| -------- | ----------- | ------ | ------- |
| kind | Type of the module. Use `Check` for checks. | Check | `Check`
| name | The name of the check. Only **unique** names allowed. | String| `service:checkname`
| about | A human readable short description of the task. | String| `checks host file for postgres`
| command | A command with which to launch the check. This has to be an array of the path to the program and its arguments. This does not invoke a shell so things like pipes (`\|`) and other shell operators will not work unless explicitly run within a shell (eg. in `bash -c`). The `shell` option described below will always run the command in a shell and should be preferred if use of shell features is required. **The check is only successful if this command exits with zero-code** | String[] | `["bash", "-c", "check-something \|\| exit 1"]`
| shell | A shell command with which to launch the check. Unlike `command` this is a cmd line string which is evaluated in a shell context (`bash`). Only **one of** `command`/`shell` must be present. **The check is only successful if this command exits with zero-code** | String | `check-something \|\| exit 1`
| help | An detailed error message to display the user instructing how to fix the issue the check is concerned with. | String | `Instructional text`
| suggested_fix | A command that the user will get asked to run, that can fix the issue this check tests for. (Optional) | SuggestedFix | [Suggested Fix](#suggested-fix-for-checks)
| working_dir | The working directory all commands and paths are relative to. Relative directories are allowed and they are relative to the location of the `cartel.yml` file. (Optional) | String | `./services/my-service`

#### Example

```
kind: Check
name: backing_services:postgres_host_file
about: postgres host file entry
shell: cat /etc/hosts | grep postgres
help: |+
  The following entry is missing from your hosts file:
    127.0.0.1 postgres
suggested_fix:
  shell: cat fixed > /tmp/fixed
  message: Details about how this is going to be fixed
```

### Environment sets
Environment sets are sets of environment variables that can be toggled on or off. They are by default **off** and have to be explicitly activated.

For example we can define two environment sets `debug` and `production`.
```
kind: Service
name: my_service
environment:
    SOME_ENV: 1
    LOG_LEVEL: INFO
environment_sets:
    debug:
        DEBUG_MODE: 1
        LOG_LEVEL: DEBUG
    production:
        LOG_LEVEL: ERROR
```
Then we can activate one or more environment sets using the `-e <env_set_name>` option.
```
$ cartel deploy my_service
# will deploy `my_service` with:
SOME_ENV=1
LOG_LEVEL=INFO

$ cartel deploy -e debug my_service
# will deploy `my_service` with:
SOME_ENV=1
DEBUG_MODE=1
LOG_LEVEL=DEBUG

$ cartel deploy -e debug -e production my_service
# will deploy `my_service` with:
SOME_ENV=1
DEBUG_MODE=1
LOG_LEVEL=ERROR
```

### Readiness and Liveness probes

**Readiness probes** are used to determine when a service is **ready** while deploying. This means services that depend on it won't deploy until its readiness checks pass.

**Liveness probes** are used to determine when a service is **healthy** after deploying. This is useful to determine the service status in `cartel ps`, or to skip deploying the service if it is already healthy.

There are three types **net**work probes, **exec**utable probes, and **log_line** probes. Together they should cover most means for checking the health of a service.

#### Net probe

Attempt to connect to the following host/port. If connection succeeds, the service is considered healthy. For more complex setups look at `exec` prob below combined with `curl`.

```
readiness_probe:
    type: net
    # number of failures before considered failed.
    retries: 10
    # The host to connect to
    host: localhost
    # The port to connect to 
    port: 8301
```

#### Executable probe

Run a command to determine the health of the service. If the command exits with `0` status code then the service is considered healthy.

```
readiness_probe:
    type: exec
    # number of failures before considered failed.
    retries: 10
    # The command to execute as the probe. Exit code zero is considered healthy.
    command: ["bash", "-c", "exit 0"]
    # Alternatively execute a command in a shell instead of a command array
    shell: exit 0
    # The working directory where the command is performed from.
    working_dir: ./my_service
```

#### Log line probe

Wait for a specific regex to match before considering the service as healthy. Only suitable as a **readiness** check.

```
readiness_probe:
    type: log_line
    # number of failures before considered failed.
    retries: 10
    # The regex to attempt to match on a log line.
    line_regex: Listening...
```


### Suggested fix for checks

A **suggested fix** may be defined for a check that a user may optionally apply.

```
suggested_fix:
  # The command to execute to fix the issue.
  command: ["bash", "-c", "echo example"]
  # Alternatively execute a command in a shell instead of a command array.
  shell: echo example
  # A message explaining to the user what the fix will do.
  message: Will add `127.0.0.1` to your /etc/hosts file.
  # The working directory where the command is performed from.
  working_dir: ./my_service
```
