# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

## [Unreleased]
### Added

### Changed

### Removed

## [7.1.0] - 2020-05-21
### Changed
- Minimal rust version was increased to 1.37.0
- `base64` dependency was bumped to 0.12

## [7.0.0] - 2019-12-20
### Added
- `std::error:Error` implementation for internal Error types: `CreationError`, `AdditionError`, `SubtractionError`, `RecordError`, `UsizeTypeTooSmall`, `DeserializeError`, `IntervalLogWriterError`, `V2DeflateSerializeError`, `V2SerializeError`
- Changelog

### Changed
- `DeserializeError` and `V2DeflateSerializeError` lost derived traits: `PartialEq`, `Eq`, `Clone`, `Copy`
- Inner error type from `std::io::ErrorKind` to `std::io::Error` in types: `DeserializeError`, `V2DeflateSerializeError`, `V2SerializeError`, `IntervalLogWriterError` to support `Display`.

[Unreleased]: https://github.com/HdrHistogram/HdrHistogram_rust/compare/v7.1.0...HEAD
[7.1.0]: https://github.com/HdrHistogram/HdrHistogram_rust/compare/v7.0.0...v7.1.0
[7.0.0]: https://github.com/HdrHistogram/HdrHistogram_rust/compare/v6.3.4...v7.0.0
