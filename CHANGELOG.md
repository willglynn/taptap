# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- initial build chain implementation
- this Changelog.md file
- implementation of reconnect logic for both serial and tcp connections
- implementation of the TCP keepalive probing mechanism
- infrastructure even in observe mode - to output in JSON gateway and nodes addresses, version and barcodes
- persistent storage for infrastructure data

### Fixed

- fixed cli argument ---port can be used together with ---tcp
- Rust compilation warnings
- duplication of main.rs code
- invalid NodeTableResponse struct

### Changed

- format of the power report event message - this is breaking change
- Updated README to reflect recent implementation
- updated Cargo deps
- cli arguments dependencies and exclusions
- README to reflects new cli arguments
