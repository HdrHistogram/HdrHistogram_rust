# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

## [Unreleased]
### Added
- `std::error:Error` implementation for internal Error types: `CreationError`, `AdditionError`, `SubtractionError`, `RecordError`, `UsizeTypeTooSmall`, `DeserializeError`, `IntervalLogWriterError`, `V2DeflateSerializeError`, `V2SerializeError`
- Changelog

### Changed
- `DeserializeError` and `V2DeflateSerializeError` lost derived traits: `PartialEq`, `Eq`, `Clone`, `Copy`
- Inner error type from `std::io::ErrorKind` to `std::io::Error` in types: `DeserializeError`, `V2DeflateSerializeError`, `V2SerializeError`, `IntervalLogWriterError` to support `Display`.

### Removed
