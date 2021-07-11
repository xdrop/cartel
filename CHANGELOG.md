# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.0-beta] - 2021-07-10
### Added
- Allow services to specify services / tasks that should always be deployed "after" them, but should not constitute a direct dependency.
- Checks can now define a `suggested_fix` section with a command that can be run, should users choose to, in order to fix the issue the check tests for.

### Changed
- `daemon restart` will start the daemon if it is not running

## [0.9.5-beta] - 2021-07-07
### Added
- Allow executable probe and shell definitions to also use the `shell` shortcut (instead of specifying a command).
- Add `daemon restart` shortcut for restarting the daemon

### Fixed
- Supress output of `stderr` from checks

## [0.9.4-beta] - 2021-07-04

### Changed
- The `always_await_readiness_probe` option will now default to `true` instead of `false`.

## [0.9.3-beta] - 2021-06-25
### Fixed
- Do not skip deployment of any tasks when `--force` is active.

## [0.9.2-beta] - 2021-06-14
### Added
- Log files for tasks can now also be viewed with `logs <task-name>`

## [0.9.1-beta] - 2021-06-14
### Fixed
- Fixed an issue where where a group's tasks would always redeploy regardless of whether the services were deployed.

## [0.9.0-beta] - 2021-06-12
### Added
- Tasks will now no longer deploy if the service is in a healthy state and running.
- Allow check definitions to also use the `shell` shortcut (instead of specifying a command).
- The `ps` command is now coloured by default making failed services more easily stand out. This may be disabled using `-n`/`--no-color`.

### Fixed
- Fixed usage of `shell` on tasks when ran with the `run <task>` option.
- Fixed a bug where the same check would be run twice when include by two different services.

## [0.8.1-beta] - 2021-05-05
### Added
- Services can now define `environment_sets` to allow configuring environment variables for a deployment. Using `-e ...`/`--env ...` as extra arguments to `deploy` can activate one or more environment sets.
- Added a new `shell` option for services and tasks (on the same level as `command`) to make invoking shell commands less verbose.

## [0.8.0-beta] - 2021-04-24
### Added
- Added support for parallel deployment. The `deploy` command will now deploy modules in parallel by default.
- Added `--threads`/`-t` to control how many threads to use to deploy in parallel. Also, added `--serial`/`-k` to force serial deployment.

### Fixed
- Fixed an issue where dependencies would be deployed in the wrong order.

## [0.7.4-beta] - 2021-04-10
### Added
- Added a new `ordered_dependencies` module option for specifying dependencies of a services that need to be run sequentially.

## [0.7.3-beta] - 2021-03-30
### Fixed
- Fixed a panic in the daemon that would cause readiness probes to always fail.

## [0.7.2-beta] - 2021-03-28
### Fixed
- Fixed an issue where the daemon would be relaunched unnessecarily.

## [0.7.1-beta] - 2021-03-28
### Added
- Added daemon launcher script for launching the daemon while opening shell terminals.


## [0.7.0-beta] - 2021-03-27
### Added
- Reworked the healthchecks system into `readiness` and `liveness` probes which can be independently used to check whether a service has started (readiness) and whether the services continues to live (liveness). Previous configuration entry of `healthcheck` has been renamed to `readiness_probe`, and a new module configuration block named `liveness_probe` has been introduced.
- Added a new option to `ps` to show liveness status.
- Allow specifying shell type to invoke using `-t`/`--type`, and adding a `type` on a shell definition.
- Improved error messages when specifying dependencies that don't exist.
- Improvements to the way the daemon is run to better support shell based environments.

### Changed
- A service with a failing liveness probe will now always be redeployed.

## [0.6.0-beta] - 2021-02-18
### Added
- Added a new healthcheck of type `net` which allows monitoring the health of a service by trying to obtain a TCP connection. The healthcheck attempts to establish a connection, which upon established will make the healthcheck succeed. On a TCP reset the healthcheck is considered failed and will also time out after 100ms.
- Added a new `-o` option to allow deploying only selected modules and not their dependencies.
- Added a new `-w`/`--wait` option for enforcing all deployed modules to pass their healthcheck before continuing.
- Added a new `shell` command and module type for defining and opening a shell to a service.

### Changed
- Renamed the module definition file extension from `.yaml` to `.yml`.

### Fixed
- Fixed an issue in working directory resolution for shell definitions.

## [0.5.0-beta] - 2021-01-24
### Changed
- Changed flag for skipping checks to `-z` and `--no-checks`.
- Improved the error message when a healthcheck failed due to misconfiguration.
- Daemon default port changed to `13754` to reduce chance of conflict.
- Changed release status to `beta`.

### Fixed
- Attempting to stop or restart a module that doesn't exist now returns an appropriate error message.
- Skip checks flag (`-z`) is now under the correct command.

### Added
- Allow forcing deployments to always redeploy using `-f` or `--force`.
- Introduced a new type of healthcheck (`log_line`) which allows for considering a service healthy when a certain line has appeared in its stdout.
- Introduced a new client configuration file to contain persisted options. 
- Add `daemon_port` option to allow configuring the daemon port via the client configuration file
- Add `default_dir` option to allow configuring the default directory to look for module definition files in. Note that this directory is always last in precedence.
- Allow skipping healthchecks using `-s` / `--no-healthchecks`.

## [0.4.2-alpha] - 2021-01-17
### Changed
- Minor improvements to wait spinners.

## [0.4.1-alpha] - 2021-01-17
### Added
- Module file can now be discovered on any path ancestral to the path the command was called.
- Modules can now be overriden with user specific settings with an overrides file (`.override.yaml'`) or using `-o <override_file>`.

### Fixed
- Fixed an issue where healthcheck would always run regardless of dependencies.

## [0.4.0-alpha] - 2021-01-16
### Fixed
- Print help when no subcommand specified.

### Changed
- Healthchecks are not awaited by default. Only when there is a dependency on the module (or a `post_up` task is defined) then the healtheck will be awaited. This behaviour may be overriden with `always_wait_healthcheck` in the module definition.

### Added
- Introduced two new options `post` and `post_up` to add tasks to be performed _after_ a service has been deployed (`post`) / or has passed its healthcheck (`post_up`) respectively (experimental).

## [0.3.3-alpha] - 2021-01-13
### Fixed
- Fixed an issue where the paths failed to resolve if `working_dir` was not specified.

### Added
- Made retries for a healthcheck configurable via the `retries` property.

## [0.3.2-alpha] - 2021-01-12
### Added
- Added aliases for most commands (eg. `deploy,d`, `restart,rr`, `run,r`, `stop,s`).
- Added two new options for logs `--all` and `--follow`.
- The path to the module definition file can now be specified with `-f` or `--file`.


## [0.3.1-alpha] - 2021-01-11
### Added
- Added the ability for the `stop` command to stop multiple services at once.
- Added a new `down` command to allow stopping every running service without having to specify names.

### Changed
- Healthcheck `kind` has been renamed to `type` and it's values are now lowercase (eg. `type: exec`).
- Increased the number of attempts for a healthcheck by 4 seconds.

### Fixed
- Paths within module definitions are now correctly resolved relative to the client rather than the daemon.
- Fixed an issue where the healthchecks command stdout will be printed on the daemon stdout.

## [0.3.0-alpha] - 2021-01-11
### Added
- Added a new `Group` declaration to allow for modules whose sole purpose is to group other modules. They can be deployed, specified as a dependency, or have checks just like other modules.
- Added cleanup for child processes on SIGINT or SIGTERM
- Services can now define a `healthcheck` section to implement checks which must pass before further dependencies are started (experimental).

## [0.2.2-alpha] - 2021-01-07
### Changed
- Changed the default pager from `less` to `tail` with a default 30 line output.

### Fixed
- Fix incorrectly labeled log files. Services now have a `.service` suffix and tasks have `.task`.

### Added
- Allow termination signal to be specified on services using `termination_signal` on the YAML config.

## [0.2.1-alpha] - 2021-01-06
### Fixed
- Switched to `SIGKILL` as the default kill signal. Fixes a race condition with contention on the log file during restarts.

## [0.2.0-alpha] - 2021-01-05
### Changed
- Increased the timeout for deploying tasks.
- Improved error message when trying to view logs of non existant service.

### Fixed
- The daemon will now attempt to also kill descendant processes (rather than only terminating the parent and leaving orphans behind). This is only implemented for Unix based systems and is not foolproof.

## [0.1.2-alpha] - 2021-01-04
### Fixed
- Fixed a bug that would cause a panic during dependency resolution.

### Changed
- The `environment` and `dependencies` options are now optional.
- The `ps` command now has better output with long service names.
- The error messages when a service fails to start are now slightly more helpful.

### Added
- Resolution for the tilde in `working_dir` paths.
- Running with no command now prints the cli help.
- Added a changelog!


## [0.1.1-alpha] - 2021-01-03
### Added
- Added `restart` command line option for restarting services that may have failed to start or are stuck.

## [0.1.0-alpha] - 2021-01-03
### Added
- Initial version of the project
