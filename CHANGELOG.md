# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.2 (2022-11-29)

### Major Changes

- Replace zwp-virtual-keyboard with wayland-protocols-misc

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 11 calendar days.
 - 11 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix clippy and Windows build issues ([`a735bd5`](https://github.com/hid-io/hid-io-core/commit/a735bd5693485b90940cbcce16bc4057aeb44621))
    - Replace zwp-virtual-keyboard with wayland-protocols-misc ([`9c048a2`](https://github.com/hid-io/hid-io-core/commit/9c048a2e06de93e6fb0a455cb6343353a00795af))
    - Fix clippy warnings ([`33c4c07`](https://github.com/hid-io/hid-io-core/commit/33c4c0751d797858b07ed395aa0bcd81cb6e9198))
</details>

## 0.1.1 (2022-11-17)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 75 commits contributed to the release over the course of 1076 calendar days.
 - 1085 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add tokio as a public crate for easier library importing ([`3f9862b`](https://github.com/hid-io/hid-io-core/commit/3f9862b8d3429142658fbdbe1b885894a5cd9ceb))
    - Adding hid-io-client ([`77e5bd6`](https://github.com/hid-io/hid-io-core/commit/77e5bd6aa17a417939fec4bfba5f8ad2f6ee7ac5))
    - Fixes for PixelSetting and DirectSet capnp rpc ([`5394d79`](https://github.com/hid-io/hid-io-core/commit/5394d79da0b9fbc4a56bc104ca468e992be1241e))
    - Adding pixelSet and pixelSetting to capnp api ([`c742cf7`](https://github.com/hid-io/hid-io-core/commit/c742cf73c7de7f835318b25578fdd9dd741f1792))
    - Add pixel tool commands ([`980dc13`](https://github.com/hid-io/hid-io-core/commit/980dc138656b30f3707421b9478fed409fa54bf4))
    - Fixing clippy warnings ([`679d47d`](https://github.com/hid-io/hid-io-core/commit/679d47dd81a3cdd5b0fe9819b150a20022cf32e5))
    - Update capnproto schema to better describe manufacturing commands ([`486d854`](https://github.com/hid-io/hid-io-core/commit/486d854936b8fc2e51b5c4d50bce9e3b3760abde))
    - Add basic manufacturing test tooling ([`70020ee`](https://github.com/hid-io/hid-io-core/commit/70020eeb5d3b9597d04027b4c030fde627eff8f4))
    - Adding basic pixelSetting and pixelSet protos ([`25f5bf3`](https://github.com/hid-io/hid-io-core/commit/25f5bf3976645936d019024d83d4f4b4f5256a6e))
    - Add h0021(pixelset) h0026(directset) and update manufacturing commands ([`9eee16d`](https://github.com/hid-io/hid-io-core/commit/9eee16da20dd2be07fd83507d229da7124c45419))
    - Add h0030_openurl and CommandInterface for modules ([`69aee41`](https://github.com/hid-io/hid-io-core/commit/69aee411e1f0daf0e1f58b601a9696c49c8ce18a))
    - Add capnproto api for h0002_test and h0000_supported_ids ([`2979ac6`](https://github.com/hid-io/hid-io-core/commit/2979ac6adbe35213e41247545e7aa48d69407b8c))
    - Renaming defmt-impl feature to defmt ([`4f85e19`](https://github.com/hid-io/hid-io-core/commit/4f85e19aa908e7698a0962051f212e068900fc8c))
    - Switching to maintained memmap2 ([`8bab0ca`](https://github.com/hid-io/hid-io-core/commit/8bab0ca65d90851a3e27cdcbfd239f8f914cd49e))
    - Increment patch ([`6db862b`](https://github.com/hid-io/hid-io-core/commit/6db862b53552d417bb875d69de9eff422264eed7))
    - Fix tempfile usage so we no longer need the world_accessible patches ([`553a585`](https://github.com/hid-io/hid-io-core/commit/553a58557f5677fc55d19ee2867414a5dd7f5023))
    - Nightly clippy issues ([`bf655ee`](https://github.com/hid-io/hid-io-core/commit/bf655ee743dbb0b7033bbaf21343beb3e5024e89))
    - Updating linux dependencies to the latest versions ([`08e975d`](https://github.com/hid-io/hid-io-core/commit/08e975d360d50524e56cc2eb249cc3bfedf74993))
    - Fix clippy and build warnings ([`6f0ad47`](https://github.com/hid-io/hid-io-core/commit/6f0ad470ca2fb8342f7febb4cdc395bc1c38c689))
    - Remove bincode-core and serde dependencies ([`8d18fe9`](https://github.com/hid-io/hid-io-core/commit/8d18fe9809e7c3abdf6c1aa8812c20be893808f6))
    - Fixing test case issue with heapless ([`d34ef8c`](https://github.com/hid-io/hid-io-core/commit/d34ef8c1a76bb8846df41ca9969e8daab50e2e7b))
    - Adding basic defmt support to hid-io-protocol ([`c074bdb`](https://github.com/hid-io/hid-io-core/commit/c074bdbb73fe55a1a44d3c690df11883453d809c))
    - Updating to heapless 0.7 ([`674e724`](https://github.com/hid-io/hid-io-core/commit/674e724ae182af6c0d99f8012f1b4d489cced3df))
    - Adding versioning to hid-io-protocol ([`589db1e`](https://github.com/hid-io/hid-io-core/commit/589db1e80208dba0599149ab7f3283ce0b49d041))
    - Adding manufacturing-test support to examples ([`3213ebe`](https://github.com/hid-io/hid-io-core/commit/3213ebee96b1686272fff4f06c735baf9e2d2e04))
    - Fixing clippy errors ([`4d5e4d7`](https://github.com/hid-io/hid-io-core/commit/4d5e4d73daa6e2de08ff0378fe82f5b87701cd93))
    - Adding h0051 manufacturing test result ([`c8ce11c`](https://github.com/hid-io/hid-io-core/commit/c8ce11c6a5f9de8c4788e569b35945ccb85d522e))
    - Fixing h0030 and h0034 terminal commands ([`d58af7e`](https://github.com/hid-io/hid-io-core/commit/d58af7edb1081fed8ee3bb27876191b389258120))
    - More bring-up with hid-io-kibohd ([`e9c2cf3`](https://github.com/hid-io/hid-io-core/commit/e9c2cf38eae556a736203b8450633a0826d3b23f))
    - Fixing Sync packets ([`b83f996`](https://github.com/hid-io/hid-io-core/commit/b83f9960bd52eb0fbb4e31acaf66296fd2b6d72c))
    - More cleanup from hid-io-protocol integration ([`166ef4b`](https://github.com/hid-io/hid-io-core/commit/166ef4bf6fa93647484438c9d66224045a8825c4))
    - Integrated h0031 and h0034 into hid-io-core ([`32956aa`](https://github.com/hid-io/hid-io-core/commit/32956aadcec61588a433a03e5173406a21f7cf38))
    - Integrating more hid-io-protocol into hid-io-core ([`ef15ee1`](https://github.com/hid-io/hid-io-core/commit/ef15ee1cb1ce8b66b68cd0eea89999b869399d98))
    - Adding more hid-io commands ([`d835b13`](https://github.com/hid-io/hid-io-core/commit/d835b13ed0bb3645c7a0c3db6691dc88c42abddb))
    - Adding h0050 and integrating h0001 and h0005 in to hid-io-core ([`c24a6ef`](https://github.com/hid-io/hid-io-core/commit/c24a6ef27a213fc4cdf11a95f494cacaba0a2691))
    - Initial integration of hid-io-protocol into hid-io-core ([`36b62e4`](https://github.com/hid-io/hid-io-core/commit/36b62e4292e3e605f80a558d9e372befe0fe1001))
    - Adding split buffer processing ([`ebde1c1`](https://github.com/hid-io/hid-io-core/commit/ebde1c15a66ca3570413ddbff7f30a9fc058ca25))
    - Splitting out hid-io-protocol into it's own crate ([`46503de`](https://github.com/hid-io/hid-io-core/commit/46503de936dded5cfe6816637d286a1f47ad864a))
    - More tools and hid-io command work ([`192444e`](https://github.com/hid-io/hid-io-core/commit/192444e2f3c164ae268f5fbdbdcf30a21ebe0041))
    - Adding Terminal hid-io packet as a generic supported id ([`d0baab1`](https://github.com/hid-io/hid-io-core/commit/d0baab1355bd40ba24dd22364eb1c8e98a4575c8))
    - Small typo in Windows deps ([`543c5ce`](https://github.com/hid-io/hid-io-core/commit/543c5ce3aee62457277b0455161b61153533e0b7))
    - Adding library entry point ([`96567e4`](https://github.com/hid-io/hid-io-core/commit/96567e4350252182185b4d0e1ec9314f2e630f1e))
    - Adding optional features for hid-io ([`90eda51`](https://github.com/hid-io/hid-io-core/commit/90eda5100569e71ec9856ea36b16621532824388))
    - Fix new clippy warnings ([`b0d2b51`](https://github.com/hid-io/hid-io-core/commit/b0d2b51549eb12a4f6ef22da1fa921a4ec8e0584))
    - Updating GitHub Actions dependencies for Wayland ([`d37abb8`](https://github.com/hid-io/hid-io-core/commit/d37abb8ae5c9b9daf3050a05c39292887b51600f))
    - Updating Windows build ([`f3d410f`](https://github.com/hid-io/hid-io-core/commit/f3d410f8959390355a9a993ef5eeb707d1c454a4))
    - Adding Wayland UTF-8 typing support ([`f897ff2`](https://github.com/hid-io/hid-io-core/commit/f897ff2a8484a358308fcdc83f6585642e633b67))
    - Adding device info capnproto api ([`ab29e66`](https://github.com/hid-io/hid-io-core/commit/ab29e66de5033451c9460a8ec384050bff86a04c))
    - Fixes repeated UTF-8 emoji bug in macOS ([`372f9f7`](https://github.com/hid-io/hid-io-core/commit/372f9f7541263cd5a8d07e80018eb6658733187b))
    - Update Unicode support for macOS ([`9749bf6`](https://github.com/hid-io/hid-io-core/commit/9749bf64ec88e1c0375432846a0d67cd20a4c0dd))
    - Converting hidapi to use read_timeout() instead of polling read() ([`a689ac7`](https://github.com/hid-io/hid-io-core/commit/a689ac7d33c909630adaafc49a920439204b526b))
    - Adding info API to daemon node ([`149d4ea`](https://github.com/hid-io/hid-io-core/commit/149d4ea93d646b46e37f77007a5b4f82ae448e4c))
    - Adding manufacturingTest command ([`e7b07d2`](https://github.com/hid-io/hid-io-core/commit/e7b07d2f07dbb2b84bf8a8df68f3d4e236f17fd4))
    - Adding unicode_string and unicode_key capnproto api functions ([`e46205b`](https://github.com/hid-io/hid-io-core/commit/e46205bc146d90e5084a5a994e860e43d90291f7))
    - Refactor of unicode module ([`8f2d59d`](https://github.com/hid-io/hid-io-core/commit/8f2d59d5db494c51662982d9d648c34872f882e7))
    - Adding daemonnode skeleton ([`7a05065`](https://github.com/hid-io/hid-io-core/commit/7a0506532d249cdce5aa65552dae8d080aaef623))
    - Adding compilation toggles for Linux specific features ([`af16a4c`](https://github.com/hid-io/hid-io-core/commit/af16a4cfec4abfc8ea37c0dfe75929bcb92e8839))
    - CLI subscriptions now working again after async refactor ([`275b82d`](https://github.com/hid-io/hid-io-core/commit/275b82d9f0f26e6297b973a8a7bac54006e8a818))
    - Fixing nodes subscriptions ([`8fa3a87`](https://github.com/hid-io/hid-io-core/commit/8fa3a87c90fabe3b14ee8a455943a5eeed74555d))
    - Renaming HIDIO to HidIo in capnproto schemas ([`1acf3c0`](https://github.com/hid-io/hid-io-core/commit/1acf3c08fa1a1d2f859c9088845908992ea1b182))
    - More tokio 0.3 fixes ([`a6a545f`](https://github.com/hid-io/hid-io-core/commit/a6a545f9df42d20bd582b676502af783ba62d277))
    - Updating more cargo packages ([`754b8bf`](https://github.com/hid-io/hid-io-core/commit/754b8bfcd6b6834bc8d62fe24c8461a1b4d7486a))
    - RpcSystem abort now working correctly ([`fbb09e3`](https://github.com/hid-io/hid-io-core/commit/fbb09e3d3c6639793066c819d86cfcd045d00d14))
    - Upgrading to tokio 0.3 ([`43cdbd4`](https://github.com/hid-io/hid-io-core/commit/43cdbd4c720f5cdd66f43cada0eedf39bdca487b))
    - Github actions fixes for Linux ([`cba9d60`](https://github.com/hid-io/hid-io-core/commit/cba9d60d0cbbaaca5dc5c9781a86d7dccdd0dfa4))
    - Added working uhid nkro and 6kro keyboard tests ([`8ec7ae2`](https://github.com/hid-io/hid-io-core/commit/8ec7ae2b0e9fecbc901f41e82a1ebe7d1d72ac2f))
    - Basic evdev monitoring (keyboard hid only) is working ([`85b551c`](https://github.com/hid-io/hid-io-core/commit/85b551c5d08b0b87edd480b414acdfdbbfbe94a3))
    - Initial vhid and evdev work on Linux ([`47fd4c4`](https://github.com/hid-io/hid-io-core/commit/47fd4c4af6fc0b6a9dfae66c4a9c0bdf4df18425))
    - Major refactor ([`79901f5`](https://github.com/hid-io/hid-io-core/commit/79901f53a55ebf9ea80ae60f5f027dc5aeed3c26))
    - Adding sleep mode and Non-Acked packets to HID-IO spec ([`e5a2145`](https://github.com/hid-io/hid-io-core/commit/e5a214538a331b4f2cdc2fa319329a65c131ff57))
    - Fixing clippy errors ([`85e46a7`](https://github.com/hid-io/hid-io-core/commit/85e46a7bcb481bade9855407a29382a74db7632e))
    - Fixes some disconnection crashes ([`6e25561`](https://github.com/hid-io/hid-io-core/commit/6e25561be942b64abb357a70fb47dafc8084becf))
    - Decreasing scan delay ([`dede17a`](https://github.com/hid-io/hid-io-core/commit/dede17a85eef57d57f4c12b10d604b56f8c99b99))
    - Adding re-connecting debug terminal support ([`95b1e53`](https://github.com/hid-io/hid-io-core/commit/95b1e538f8ee4e2b58cb99ca104a335afe9e086c))
    - Cleaning up non-logged messages ([`a7171cb`](https://github.com/hid-io/hid-io-core/commit/a7171cb1dc022bcded41991d8813e5d8e5fd4704))
</details>

## v0.1.0-beta3 (2019-11-28)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 25 calendar days.
 - 37 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Incrementing to v0.1.0-beta3 ([`55d2e0a`](https://github.com/hid-io/hid-io-core/commit/55d2e0a51cc30758ba93569aac9eff7a5b4e5fb0))
    - Add log files ([`f3505be`](https://github.com/hid-io/hid-io-core/commit/f3505beefcac012f78cc5239d8019f4c4a0976be))
    - Adding on_nodesupdate callback to Python API ([`93abe49`](https://github.com/hid-io/hid-io-core/commit/93abe49d83d529281e30a812cf22c0b8e1702b29))
    - Cleanup and fixes ([`c2efaeb`](https://github.com/hid-io/hid-io-core/commit/c2efaeb437f2dbfd6decef59df2afb61e362d698))
</details>

## v0.1.0-beta2 (2019-10-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 8 calendar days.
 - 8 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Renaming top-level code sections to hid-io-core ([`74569ba`](https://github.com/hid-io/hid-io-core/commit/74569ba3c5554d436d296a833b03335cfd26b34f))
    - Updating python client library ([`9a6e2d1`](https://github.com/hid-io/hid-io-core/commit/9a6e2d13a04a65cafc37f19649d77434bbcb7924))
    - Adding working tempfile support for unix-like systems ([`26d66f8`](https://github.com/hid-io/hid-io-core/commit/26d66f82c4f8ab2273d74de680b8bf98a8aa6851))
</details>

## v0.1.0-beta1 (2019-10-13)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 91 calendar days.
 - 190 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - GitHub Actions ([`6a7fa76`](https://github.com/hid-io/hid-io-core/commit/6a7fa76675fb46a548e28d116a270cb8a7022a38))
    - Updating to new authentication scheme ([`3d18c78`](https://github.com/hid-io/hid-io-core/commit/3d18c784b72ecf4ad7502484a5564500b55c6289))
    - Fixing Windows arg bug ([`5460e7b`](https://github.com/hid-io/hid-io-core/commit/5460e7b174cac719630c0e4ce8aa974e54e85e93))
    - Fixing Windows linting errors ([`2a8bfda`](https://github.com/hid-io/hid-io-core/commit/2a8bfda94582d04300161c6af81efafd69c54841))
    - Fixing Linux linter warnings ([`591e811`](https://github.com/hid-io/hid-io-core/commit/591e81168ef6762a12ef043e02811a907e9f0d33))
    - Updating dependencies to work with rust nightlyg ([`b882107`](https://github.com/hid-io/hid-io-core/commit/b882107a4c3a6794cb7675fa5565a24d0d7acfac))
</details>

## v0.1.0-beta (2019-04-05)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 30 commits contributed to the release over the course of 669 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Cargo fmt ([`2ed012f`](https://github.com/hid-io/hid-io-core/commit/2ed012f95f6a5571062e6d6249b2c7d1270d081e))
    - X11 Unicode fixes ([`33aba7b`](https://github.com/hid-io/hid-io-core/commit/33aba7b17843cb02d9ed977d279b7369091dadcd))
    - More stable rpc ([`cdcb5fc`](https://github.com/hid-io/hid-io-core/commit/cdcb5fcd083bbd0c3a47b11b7297877189cca9fa))
    - No more clippy warnings ([`e6c03d1`](https://github.com/hid-io/hid-io-core/commit/e6c03d1915a5e385303bdf1a77d6f37eb922c3cb))
    - More docs ([`37e945a`](https://github.com/hid-io/hid-io-core/commit/37e945ab58b03fd46c3d3929698ac5ae3eb6e656))
    - Update packet id's to match spec ([`247b4a3`](https://github.com/hid-io/hid-io-core/commit/247b4a39732981f29f7f2f8e43df277af1100209))
    - Cleanup ([`10d187d`](https://github.com/hid-io/hid-io-core/commit/10d187d8c23e9a9bb9601f01aacd1b6269318ab7))
    - OSX Unicode support and service ([`6d8c6d8`](https://github.com/hid-io/hid-io-core/commit/6d8c6d8d3f6ab5355eaf978d045e9c1d54f3b97b))
    - Windows service and cross compiling ([`64d1577`](https://github.com/hid-io/hid-io-core/commit/64d157734ce163f93f845d3847e24add5164eb95))
    - The commit ([`695c4cc`](https://github.com/hid-io/hid-io-core/commit/695c4ccd2b70772b29a11c31fd743d6c0cb385cc))
    - Collect node info durring auth ([`0e4ce23`](https://github.com/hid-io/hid-io-core/commit/0e4ce23d3893e0f8f720cccce585ef9def9f4bab))
    - Dynamic nodes list ([`f3daf71`](https://github.com/hid-io/hid-io-core/commit/f3daf7122fc6fc11736e05ff920ea952b14652c7))
    - Node storage & retrieval ([`1703c4a`](https://github.com/hid-io/hid-io-core/commit/1703c4ac2388e060dc1f484c5e5e934a7b62a643))
    - Basic auth stubs ([`3413ef2`](https://github.com/hid-io/hid-io-core/commit/3413ef2bf35ba48073d16d5b0ee756e77616c70c))
    - Add optional TLS encryption ([`1e42f56`](https://github.com/hid-io/hid-io-core/commit/1e42f566111bd17c1880475db71dba00c64dc9d8))
    - Upgrade to 2018 edition ([`2975e13`](https://github.com/hid-io/hid-io-core/commit/2975e13c9c3d945e264aa18c3ad65e81dcd77f49))
    - Create unicode x11 output module ([`f8b61c6`](https://github.com/hid-io/hid-io-core/commit/f8b61c6f574b1807c380902790cdb590f39bcff2))
    - Initial rust rpc server ([`9de2051`](https://github.com/hid-io/hid-io-core/commit/9de2051d064e6a059d77403a9a488c4ebb19e249))
    - Run rustfmt ([`7858e89`](https://github.com/hid-io/hid-io-core/commit/7858e89c3d1eb3272a835c926349afd18b437048))
    - Run rstfmt on the codebase ([`13069e8`](https://github.com/hid-io/hid-io-core/commit/13069e86152bfb4f5dbf5ee05bb8c45c8d6e9895))
    - Fixing compilation errors ([`61eeccf`](https://github.com/hid-io/hid-io-core/commit/61eeccf4111a1d97177252ccf6e93333d76bb108))
    - Missing file ([`025f91f`](https://github.com/hid-io/hid-io-core/commit/025f91fe430f9d95d66d06edaabff190cae6ef7e))
    - Example client/server in Python and initial Cap'n'Proto Schema files ([`37a9966`](https://github.com/hid-io/hid-io-core/commit/37a996697127bd7bf6d2d2ed1382dac2131126e9))
    - Adding basic cli options ([`b3427bf`](https://github.com/hid-io/hid-io-core/commit/b3427bf259a91c84d81c554a5355a921c8b14df8))
    - Adding HID-IO protocol module ([`f0bbe92`](https://github.com/hid-io/hid-io-core/commit/f0bbe9241f2720a078f68c7368e119b3bf351048))
    - Merge branch 'master' of https://github.com/hid-io/hid-io ([`afc44da`](https://github.com/hid-io/hid-io-core/commit/afc44da939defdd9cd1bb20232abf19f29937c5a))
    - Initial serialization of HID-IO packets (including continuation chunking) ([`5e04e99`](https://github.com/hid-io/hid-io-core/commit/5e04e990a82c830bf3234e038e5c6ec4ad060a7f))
    - Adding Travis CI build for Linux and macOS; appveyor for Windows ([`2ca24e2`](https://github.com/hid-io/hid-io-core/commit/2ca24e2b0b1302aaf15b9e757c3f9725b6941b66))
    - Device connect/reconnect code now working ([`bfa3e15`](https://github.com/hid-io/hid-io-core/commit/bfa3e15a05ded3f987fd6f067965741e6741453b))
    - Initial code structure dump ([`d9b2931`](https://github.com/hid-io/hid-io-core/commit/d9b2931f74fdbab7d9ee5ee43356e2fa89754c96))
</details>

