# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2-alpha] - 2020-01-07
### Changed
- Changed the default pager from `less` to `tail` with a default 30 line output.

### Fixed
- Fix incorrectly labeled log files. Services now have a `.service` suffix and tasks have `.task`.

### Added
- Allow termination signal to be specified on services using `termination_signal` on the YAML config.

## [0.2.1-alpha] - 2020-01-06
### Fixed
- Switched to `SIGKILL` as the default kill signal. Fixes a race condition with contention on the log file during restarts.

## [0.2.0-alpha] - 2020-01-05
### Changed
- Increased the timeout for deploying tasks.
- Improved error message when trying to view logs of non existant service.

### Fixed
- The daemon will now attempt to also kill descendant processes (rather than only terminating the parent and leaving orphans behind). This is only implemented for Unix based systems and is not foolproof.

## [0.1.2-alpha] - 2020-01-04
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


## [0.1.1-alpha] - 2020-01-03
### Added
- Added `restart` command line option for restarting services that may have failed to start or are stuck.

## [0.1.0-alpha] - 2020-01-03
### Added
- Initial version of the project
