# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1](https://github.com/nicolasmelo1/software-factory/compare/v0.4.0...v0.4.1) - 2026-09-12

### Added

- *(check)* record how gates were unblocked
- *(L3)* a gate's criteria document stays out of the queue of undone work
- *(docs)* generate documentation from source

### Fixed

- *(overlay)* derive the inapplicable set from the check kind ([#53](https://github.com/nicolasmelo1/software-factory/pull/53))
- *(init)* the python schema query listed its fields out of grammar order ([#43](https://github.com/nicolasmelo1/software-factory/pull/43))
- *(release-plz)* stage every derivative the release PR produces
- *(check)* satisfy local CI guardrails
- *(L2)* compare policy direction on the options a rule actually runs under
- *(init)* enable and prove template rules on a fresh install
- satisfy docs lint

### Other

- *(release-plz)* open the release PR with a GitHub App token
- automate release PRs with release-plz
- *(plans)* retire the two plans whose work shipped
- merge origin/main into feat/escape-log
- reseal adoption evidence after the init change
- a small README and a generated reference
- finish documentation convention
- reseal adoption evidence
- document marker syntax safely
- *(plans)* governing from outside, generated docs, and the archive
- *(plans)* a proof path named a file that does not exist
- *(readme)* the rule counts had drifted
- *(plans)* a gate records how it was unblocked
