# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2023-08-11

### Added

- `impl From<AssetInfo> for Asset { ... }` to convert `AssetInfo` to `Asset` with zero amount.
- `fn query_asset_info_balances` method on `AssetList` to query balances of a `Vec<AssetInfo>`.
