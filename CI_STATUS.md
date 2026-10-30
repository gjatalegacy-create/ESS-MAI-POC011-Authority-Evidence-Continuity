# POC011 compiler compatibility and CI

## Current source correction

The maintained branch replaces the deprecated `fetch_update` call with a checked `compare_exchange_weak` loop. After observing counter `n`, it attempts `n -> n+1` only when `checked_add(1)` succeeds. Only a successful atomic exchange returns the allocated value; contention reloads the actual value and recomputes the checked increment. At `u64::MAX` it returns `None` without wrapping. The original relaxed ordering for this process-local identifier allocation is preserved.

This is a source compatibility correction, separate from the historical-memory correction in `67e899a`. The package keeps `warnings = "deny"`, the same dependency lock and the exact extracted `source_ffi.rs` / `zero_noi.rs` bytes. Authority duties, binding checks, native single-use consumption, canonical order and the five-cycle model are unchanged. A direct rename to `try_update` would require a newer API than the package's declared Rust 1.89 minimum; the compare-exchange operations were already available there. Validation below is limited to the toolchains actually run, not a claim that every intermediate compiler has been tested. See the [official AtomicU64 API](https://doc.rust-lang.org/stable/core/sync/atomic/type.AtomicU64.html).

Two new regressions exercise 32 concurrent callers: 2,048 allocations must be unique and contiguous; at exhaustion exactly one caller receives `u64::MAX`, the other 31 receive `None`, and the counter never wraps.

Local Windows GNU validation passed on **Rust 1.98.0 and 1.99.1**: **48 Rust tests in debug and 48 in release** for each toolchain, release build, five cycles (`0,0,0,1,1` with two durable effects), journal inspection and the independent verifier's 17 self-tests. [Verification record and logs](evidence/rust-compatibility/verification.json). The unchanged `67e899a` source was first reproduced failing compilation with 1.99.1, exit 101. An intermediate verification correctly refused the changed wrapper while its provenance hash was still stale; the hash and manifest were refreshed before the successful complete runs. These records remain separate from the original archive results.

The [reproduction workflow](https://github.com/gjatalegacy-create/ESS-MAI-POC011-Authority-Evidence-Continuity/actions/workflows/reproduce.yml) now runs six jobs: **1.98.0, 1.99.1 and moving stable, each on Ubuntu and Windows**. It records `rustc -Vv`, verifies the archived fixture and current manifest, and runs the same full reproduction. A later change in stable can fail visibly without weakening warnings or changing the pinned checks. The workflow page and commit check status provide hosted results; a workflow definition or Pages deployment alone is not evidence of successful tests.

To reproduce the maintained branch, use Rust 1.99.1, cache locked dependencies, then run `python tools/reproduce.py new-check --target-dir target`. This creates a new evidence directory and build artifacts. Offline tests require cached dependencies. The immutable **v0.1.0 archive remains unchanged and requires its recorded Rust 1.98.0 environment**; it does not contain either maintenance source correction.

## Historical pinning erratum (before the source correction)

The immutable `v0.1.0` release archive records local validation with Rust/Cargo 1.98.0. Its original GitHub workflow selected the moving `stable` toolchain, which selected 1.99.1 on 30 October 2026. On both Ubuntu and Windows, compilation then failed before test execution because `src/ffi.rs:25` uses `AtomicU64::fetch_update`, deprecated in that toolchain in favour of `try_update`. The package's existing `warnings = "deny"` policy converted the deprecation into an error.

The cause was exposed in [diagnostic run 41076328410](https://github.com/gjatalegacy-create/ESS-MAI-POC011-Authority-Evidence-Continuity/actions/runs/41076328410). This is a demonstrated compiler compatibility failure; it is not a failure of one of the 38 test assertions and does not erase the recorded local results.

The CI maintenance change pins **Rust 1.98.0**, retains captured diagnostics in the job log, and corrects the README's overly broad “1.89 or newer” instruction. It does not alter runtime source, tests, dependencies, thresholds, or `warnings = "deny"`. For local reproduction, select the same toolchain before the documented commands:

```text
rustup toolchain install 1.98.0 --profile minimal --component rustfmt
rustup override set 1.98.0
```

Both hosted platforms **passed** the complete pinned-toolchain reproduction in [run 41077100247](https://github.com/gjatalegacy-create/ESS-MAI-POC011-Authority-Evidence-Continuity/actions/runs/41077100247), commit `6695a96c5f1833f92d891d1310560dbf1b0c6529`: Ubuntu in 36 seconds and Windows in 56 seconds. The workflow checked the manifest and recorded evidence, ran the same 38 Rust tests in debug and release, built the release binary, executed the five-cycle demonstration and ran the Python verifier. These job durations are not runtime benchmarks.

Subsequent hosted outcomes are visible in [the reproduction workflow](https://github.com/gjatalegacy-create/ESS-MAI-POC011-Authority-Evidence-Continuity/actions/workflows/reproduce.yml). A successful Pages deployment is not a successful test run. A hosted execution is additional reproducibility evidence, not independent peer review.

The `v0.1.0` tag, its original ZIP, its checksum and its historical CI failures remain unchanged. That pinning correction alone did not establish newer-toolchain compatibility. `SHA256SUMS` covers the maintained branch's files, while the original release checksum continues to identify the frozen release. The subsequent source correction and its separate validation are described above; no new release is asserted.
