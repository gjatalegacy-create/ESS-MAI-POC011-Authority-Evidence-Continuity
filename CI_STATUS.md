# POC011 CI compatibility erratum

The immutable `v0.1.0` release archive records local validation with Rust/Cargo 1.98.0. Its original GitHub workflow selected the moving `stable` toolchain, which selected 1.99.1 on 30 October 2026. On both Ubuntu and Windows, compilation then failed before test execution because `src/ffi.rs:25` uses `AtomicU64::fetch_update`, deprecated in that toolchain in favour of `try_update`. The package's existing `warnings = "deny"` policy converted the deprecation into an error.

The cause was exposed in [diagnostic run 41076328410](https://github.com/gjatalegacy-create/ESS-MAI-POC011-Authority-Evidence-Continuity/actions/runs/41076328410). This is a demonstrated compiler compatibility failure; it is not a failure of one of the 38 test assertions and does not erase the recorded local results.

The CI maintenance change pins **Rust 1.98.0**, retains captured diagnostics in the job log, and corrects the README's overly broad “1.89 or newer” instruction. It does not alter runtime source, tests, dependencies, thresholds, or `warnings = "deny"`. For local reproduction, select the same toolchain before the documented commands:

```text
rustup toolchain install 1.98.0 --profile minimal --component rustfmt
rustup override set 1.98.0
```

Current hosted outcomes are visible in [the reproduction workflow](https://github.com/gjatalegacy-create/ESS-MAI-POC011-Authority-Evidence-Continuity/actions/workflows/reproduce.yml). A successful Pages deployment is not a successful test run. A hosted execution is additional reproducibility evidence, not independent peer review.

The `v0.1.0` tag, its original ZIP, its checksum and its historical CI failures remain unchanged. The current branch includes this maintenance documentation and the pinned workflow; it must not be described as byte-identical to the original release archive. `SHA256SUMS` covers the current branch's files, while the original release checksum continues to identify the frozen release. No newer-toolchain support or new release is asserted by this correction.
