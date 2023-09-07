# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.3 (2023-09-07)

### Bug Fixes

 - <csr-id-c63371c87e2373f2d6af3767bb80f682139e6b08/> Only use api feature from hid-io-core in hid-io-client
   - hid-io-client should not have access to control display server,
     capture hid devices or communicate with hid endpoints

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Only use api feature from hid-io-core in hid-io-client ([`c63371c`](https://github.com/hid-io/hid-io-core/commit/c63371c87e2373f2d6af3767bb80f682139e6b08))
</details>

## 0.1.2 (2023-09-07)

### Bug Fixes

 - <csr-id-78b443f8a607e23b1630fe657afb13f0acf74a0e/> Adjust README.md after dependency fixes
 - <csr-id-ceb5c43ed0c208c30f38ee01fd0997fb1a7e0d85/> Expose hid-io-client hid-io-core,capnp dependencies
   - Simpler dependency management for users of hid-io-client

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hid-io-client v0.1.2 ([`0e4321c`](https://github.com/hid-io/hid-io-core/commit/0e4321c0ff7010823c239e1c3d2e4a0904f4b987))
    - Adjust README.md after dependency fixes ([`78b443f`](https://github.com/hid-io/hid-io-core/commit/78b443f8a607e23b1630fe657afb13f0acf74a0e))
    - Expose hid-io-client hid-io-core,capnp dependencies ([`ceb5c43`](https://github.com/hid-io/hid-io-core/commit/ceb5c43ed0c208c30f38ee01fd0997fb1a7e0d85))
</details>

## 0.1.1 (2023-09-07)

### Bug Fixes

<csr-id-559757292afa1cb1e7a8d0ee28d75a3ae8a26ab2/>

 - <csr-id-62af0b510a7399645469e72f10fbfeffdb5edc7a/> Update dependencies and small fixes
   - Fix hid-io-client example tool pixel direct range

### New Features

<csr-id-6d44300e247b0e74459c8e2ad54061b5346a01ce/>

 - <csr-id-87cd06d6ea76bebb924629d86fb78fa5b9f67fe2/> Add hall effect manu test data tracking
   - Supports partial strobe data updates (only printing after getting
   enough data for a full scan)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 282 calendar days.
 - 294 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hid-io-client v0.1.1 ([`f1bdda2`](https://github.com/hid-io/hid-io-core/commit/f1bdda27b3daff27f681f680a014cc21501f057d))
    - Release hid-io-protocol v0.1.6, hid-io-core v0.1.4 ([`42068a7`](https://github.com/hid-io/hid-io-core/commit/42068a7989235bbc28888d1c705a425da26ec5fd))
    - Add hall effect manu test data tracking ([`87cd06d`](https://github.com/hid-io/hid-io-core/commit/87cd06d6ea76bebb924629d86fb78fa5b9f67fe2))
    - Add levelcheck column and mode set commands to hid-io-core + capnp ([`6d44300`](https://github.com/hid-io/hid-io-core/commit/6d44300e247b0e74459c8e2ad54061b5346a01ce))
    - Release hid-io-protocol v0.1.5, hid-io-core v0.1.3 ([`95088fc`](https://github.com/hid-io/hid-io-core/commit/95088fc5e913226d1f55b3d83ec8a7553b916368))
    - Update dependencies and small fixes ([`62af0b5`](https://github.com/hid-io/hid-io-core/commit/62af0b510a7399645469e72f10fbfeffdb5edc7a))
    - Latest clippy warnings (format string identifiers) ([`5597572`](https://github.com/hid-io/hid-io-core/commit/559757292afa1cb1e7a8d0ee28d75a3ae8a26ab2))
    - Release hid-io-protocol v0.1.4, hid-io-core v0.1.2 ([`6906d29`](https://github.com/hid-io/hid-io-core/commit/6906d29ea854e02dbf58ef6531b4468362c0abb3))
</details>

<csr-unknown>
flexi_logger 0.24 -> 0.25uhid-virt 0.0.5 -> official 0.0.6clippy fixes<csr-unknown/>

## 0.1.0 (2022-11-17)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 15 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hid-io-core v0.1.1, hid-io-client v0.1.0 ([`cd719ea`](https://github.com/hid-io/hid-io-core/commit/cd719eab05608bced35ace8f2f41f815631fca29))
    - Initial CHANGELOG.md ([`173872e`](https://github.com/hid-io/hid-io-core/commit/173872ec9e1a2d0dfc95e607f0a6abc250947e29))
    - Typo ([`3f1db58`](https://github.com/hid-io/hid-io-core/commit/3f1db58dd0fd72aa9a1f3748cd15d7a7a810e525))
    - Update hid-io-client README.md ([`400b754`](https://github.com/hid-io/hid-io-core/commit/400b75453d21cb7e40cb93b68a4c78d7fc4468e2))
    - Adding hid-io-client ([`77e5bd6`](https://github.com/hid-io/hid-io-core/commit/77e5bd6aa17a417939fec4bfba5f8ad2f6ee7ac5))
</details>

