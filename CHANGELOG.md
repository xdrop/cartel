# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2-alpha] - 2020-01-04
### Fixed
- Fixed a bug that would cause a panic during dependency resolution.

### Changed
- The `environment` and `dependencies` options are now optional.
- The `ps` command now has better output with long service names.

### Added
- Resolution for the tilde in working_dir paths.
- Running with no command now prints the cli help.
- Added a changelog!


## [0.1.1-alpha] - 2020-01-03
### Added
- Added `restart` command line option for restarting services that may have failed to start or are stuck.

## [0.1.0-alpha] - 2020-01-03
### Added
- Initial version of the project