# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.3 (2022-11-17)

### Other

 - <csr-id-7fc1f117f4d060368aac0b26e232bfab123009ce/> Fix links

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 59 commits contributed to the release over the course of 663 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add tokio as a public crate for easier library importing ([`3f9862b`](https://github.com/hid-io/hid-io-core/commit/3f9862b8d3429142658fbdbe1b885894a5cd9ceb))
    - Fixes for PixelSetting and DirectSet capnp rpc ([`5394d79`](https://github.com/hid-io/hid-io-core/commit/5394d79da0b9fbc4a56bc104ca468e992be1241e))
    - Small name cleanup ([`44698c1`](https://github.com/hid-io/hid-io-core/commit/44698c12bd4ac48a59aef6b4c4532d296493ea21))
    - Fix defmt issuse with unions ([`f133d25`](https://github.com/hid-io/hid-io-core/commit/f133d25ed0ce3ac44ed09b43aa8a6b8b76930dac))
    - Fixing clippy warnings ([`679d47d`](https://github.com/hid-io/hid-io-core/commit/679d47dd81a3cdd5b0fe9819b150a20022cf32e5))
    - Add basic manufacturing test tooling ([`70020ee`](https://github.com/hid-io/hid-io-core/commit/70020eeb5d3b9597d04027b4c030fde627eff8f4))
    - Adding basic pixelSetting and pixelSet protos ([`25f5bf3`](https://github.com/hid-io/hid-io-core/commit/25f5bf3976645936d019024d83d4f4b4f5256a6e))
    - Add h0021(pixelset) h0026(directset) and update manufacturing commands ([`9eee16d`](https://github.com/hid-io/hid-io-core/commit/9eee16da20dd2be07fd83507d229da7124c45419))
    - Add h0030_openurl and CommandInterface for modules ([`69aee41`](https://github.com/hid-io/hid-io-core/commit/69aee411e1f0daf0e1f58b601a9696c49c8ce18a))
    - Typo ([`f680f3c`](https://github.com/hid-io/hid-io-core/commit/f680f3cff6b837068cb77253f7d6db4427bde744))
    - Missing entries from open url changes ([`2152dc7`](https://github.com/hid-io/hid-io-core/commit/2152dc7ba641ccbeb993ea1c2afd1aaf1fe058ca))
    - Add 0x30 Open URL to spec ([`0e6b582`](https://github.com/hid-io/hid-io-core/commit/0e6b58245f7c1a2b6ef6ecc5cdaf2d24aa400378))
    - Fix clippy warnings ([`be2a327`](https://github.com/hid-io/hid-io-core/commit/be2a327eb9a252561ec1bb45647088253a8b29f3))
    - Increment version for feature change ([`1fd1b12`](https://github.com/hid-io/hid-io-core/commit/1fd1b1246c4de4d5333e13efd1cffcaba7fb9386))
    - Renaming defmt-impl feature to defmt ([`4f85e19`](https://github.com/hid-io/hid-io-core/commit/4f85e19aa908e7698a0962051f212e068900fc8c))
    - Re-adding defmt support to hid-io-protocol + kll-core ([`e6be9ae`](https://github.com/hid-io/hid-io-core/commit/e6be9aef9dba3c79325f07cf8107665e211ce470))
    - Increment patch ([`6db862b`](https://github.com/hid-io/hid-io-core/commit/6db862b53552d417bb875d69de9eff422264eed7))
    - Fix clippy warnings ([`919649d`](https://github.com/hid-io/hid-io-core/commit/919649d88df99541da4f7e9004e14e72e91acc88))
    - Nightly clippy issues ([`bf655ee`](https://github.com/hid-io/hid-io-core/commit/bf655ee743dbb0b7033bbaf21343beb3e5024e89))
    - Add h0020_klltrigger support ([`5e465e7`](https://github.com/hid-io/hid-io-core/commit/5e465e7b00119a8ad22f18414cf4baf5674d2f19))
    - Update README.md ([`8959ebc`](https://github.com/hid-io/hid-io-core/commit/8959ebce7b7b40e4ff269bf2f93c6eb6cdc640ac))
    - Remove bincode-core and serde dependencies ([`8d18fe9`](https://github.com/hid-io/hid-io-core/commit/8d18fe9809e7c3abdf6c1aa8812c20be893808f6))
    - Fixing test case issue with heapless ([`d34ef8c`](https://github.com/hid-io/hid-io-core/commit/d34ef8c1a76bb8846df41ca9969e8daab50e2e7b))
    - Updating to 2021 edition ([`04bb40f`](https://github.com/hid-io/hid-io-core/commit/04bb40f7959e7a810672447d02701976853fe0f5))
    - Adding basic defmt support to hid-io-protocol ([`c074bdb`](https://github.com/hid-io/hid-io-core/commit/c074bdbb73fe55a1a44d3c690df11883453d809c))
    - Removing unused dependency ([`e08053f`](https://github.com/hid-io/hid-io-core/commit/e08053fd2599f15150a26acf8bd4d8173e9732bc))
    - Updating to heapless 0.7 ([`674e724`](https://github.com/hid-io/hid-io-core/commit/674e724ae182af6c0d99f8012f1b4d489cced3df))
    - Adding versioning to hid-io-protocol ([`589db1e`](https://github.com/hid-io/hid-io-core/commit/589db1e80208dba0599149ab7f3283ce0b49d041))
    - Merge pull request #20 from half-duplex/spec-links ([`7370812`](https://github.com/hid-io/hid-io-core/commit/7370812e08352d82db1e1e7776505ed0c59640ea))
    - Fix links ([`7fc1f11`](https://github.com/hid-io/hid-io-core/commit/7fc1f117f4d060368aac0b26e232bfab123009ce))
    - Adding manufacturing-test support to examples ([`3213ebe`](https://github.com/hid-io/hid-io-core/commit/3213ebee96b1686272fff4f06c735baf9e2d2e04))
    - Update README.md ([`a32bee9`](https://github.com/hid-io/hid-io-core/commit/a32bee9e1bf39c50ed3afc8cb3775da0292ad414))
    - Moving hid-io-kiibohd to kiibohd-core ([`ab47ec0`](https://github.com/hid-io/hid-io-core/commit/ab47ec012257dd04aa25e86d0ff9b93eb3511962))
    - Fixing clippy errors ([`4d5e4d7`](https://github.com/hid-io/hid-io-core/commit/4d5e4d73daa6e2de08ff0378fe82f5b87701cd93))
    - Update README.md ([`de0578c`](https://github.com/hid-io/hid-io-core/commit/de0578c0e59c9ca8252b641008101faee3482ec3))
    - Adding h0051 manufacturing test result ([`c8ce11c`](https://github.com/hid-io/hid-io-core/commit/c8ce11c6a5f9de8c4788e569b35945ccb85d522e))
    - Update README.md ([`8710a10`](https://github.com/hid-io/hid-io-core/commit/8710a10f08d22acd03aded8ae0dd4afbf9689604))
    - Update README.md ([`d3e7842`](https://github.com/hid-io/hid-io-core/commit/d3e7842fd42b84de6397dead44fb1b699665ba46))
    - Fixing h0030 and h0034 terminal commands ([`d58af7e`](https://github.com/hid-io/hid-io-core/commit/d58af7edb1081fed8ee3bb27876191b389258120))
    - Updating README with some usage information. ([`536759d`](https://github.com/hid-io/hid-io-core/commit/536759d236c4dc9255a474b9b7f5a5df865d9ee0))
    - Starting libhid_io_kiibohd.a integration with kiibohd/controller ([`d4e4ed7`](https://github.com/hid-io/hid-io-core/commit/d4e4ed72e52e88bd5aafcdab76320c450f953ecb))
    - Fixing Sync packets ([`b83f996`](https://github.com/hid-io/hid-io-core/commit/b83f9960bd52eb0fbb4e31acaf66296fd2b6d72c))
    - Fixes to get GitHub Actions passing again ([`e0cb9ab`](https://github.com/hid-io/hid-io-core/commit/e0cb9ab95c1f26e02ff3f863a170d068cb6edb88))
    - Adding sync and no payload data serialization/deserialization tests ([`7b3c25c`](https://github.com/hid-io/hid-io-core/commit/7b3c25cafc1a713b36da0458f8a4c3c479e00a16))
    - hid-io-kiibohd additions ([`73fd32e`](https://github.com/hid-io/hid-io-core/commit/73fd32e0c9f90d130d6cf6d3412c54b188301f8a))
    - Adding more commands to hid-io-kiibohd ([`7860399`](https://github.com/hid-io/hid-io-core/commit/786039953c657af9658be32ff74941bc79f889fb))
    - Integrated h0031 and h0034 into hid-io-core ([`32956aa`](https://github.com/hid-io/hid-io-core/commit/32956aadcec61588a433a03e5173406a21f7cf38))
    - Adding more hid-io commands ([`d835b13`](https://github.com/hid-io/hid-io-core/commit/d835b13ed0bb3645c7a0c3db6691dc88c42abddb))
    - Adding h0050 and integrating h0001 and h0005 in to hid-io-core ([`c24a6ef`](https://github.com/hid-io/hid-io-core/commit/c24a6ef27a213fc4cdf11a95f494cacaba0a2691))
    - Initial integration of hid-io-protocol into hid-io-core ([`36b62e4`](https://github.com/hid-io/hid-io-core/commit/36b62e4292e3e605f80a558d9e372befe0fe1001))
    - Adding basic support for NAData packets ([`bfe49ee`](https://github.com/hid-io/hid-io-core/commit/bfe49ee5c189b2583441e43e25ec83526fb1dbf5))
    - Adding split buffer processing ([`ebde1c1`](https://github.com/hid-io/hid-io-core/commit/ebde1c15a66ca3570413ddbff7f30a9fc058ca25))
    - Added invalid id test ([`43fd7b3`](https://github.com/hid-io/hid-io-core/commit/43fd7b381c647841f82b1d8f1dded66713868cdc))
    - Adding h0003 skeleton ([`1962b55`](https://github.com/hid-io/hid-io-core/commit/1962b557effa17ec95786b989b8b8a6e8eef00bf))
    - Splitting commands.rs to mod.rs and test.rs ([`d2bdbb7`](https://github.com/hid-io/hid-io-core/commit/d2bdbb71c7bbd6f96a0dae02dc98f402cf57fcda))
    - Adding h0002 (test packet) ([`450174b`](https://github.com/hid-io/hid-io-core/commit/450174bd4a2b1bd6bf75e24562646654caf8d0ce))
    - h0001 - Get Info added ([`758d03e`](https://github.com/hid-io/hid-io-core/commit/758d03e208ecdccb33dd1909000d9e6751ad1ba3))
    - First test case working ([`833ea59`](https://github.com/hid-io/hid-io-core/commit/833ea5911cc41f541df075bec1221cd84066db60))
    - Splitting out hid-io-protocol into it's own crate ([`46503de`](https://github.com/hid-io/hid-io-core/commit/46503de936dded5cfe6816637d286a1f47ad864a))
</details>

