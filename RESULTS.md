# POC011 — executed results

Current-branch maintenance results are recorded in [MEMORY_INTEGRITY_FIX.md](MEMORY_INTEGRITY_FIX.md). The original execution results below and `evidence/final` remain the frozen `v0.1.0` evidence. The original 38 tests did not cover the rehashed historical-reference inconsistencies found in the subsequent audit.

Final validation completed 2026-10-30 on Windows 10 AMD64, Cargo 1.98.0. The authoritative run is **`evidence/final/`**. This is a local reproducibility result for the bounded capsule, not independent peer review or certification of ESS-MAI, AIGovern-Cyber or VERITAS.

## Results

| Check | Executed result |
| --- | --- |
| `cargo fmt --all -- --check` | Exit 0 |
| `cargo test --locked --offline` | 38 passed: 33 library + 3 CLI + 2 process integration; 0 failed, 0 ignored |
| `cargo test --locked --offline --release` | The same 38 tests passed; 0 failed, 0 ignored |
| `cargo build --locked --offline --release` | Exit 0 |
| Release five-cycle demonstration | Exit 0; five scalar results `0,0,0,1,1` |
| Read-only journal inspection | Exit 0; five records, two unique effects |
| Independent Python verifier | PASS; 17 verifier self-tests passed |
| Source provenance | All available source/extraction hashes checked; no missing original sources on this host |

The test builds use `warnings = "deny"`; no compiler warnings were emitted. Debug and release are two executions of the same tests, not 76 distinct properties. Cargo reports zero doctests; no doctest coverage is claimed.

Exact commands, paths, return codes and durations: [commands.json](evidence/final/commands.json). Environment: [environment.json](evidence/final/environment.json). Detailed logs: [debug](evidence/final/test-debug.stdout.log), [release](evidence/final/test-release.stdout.log), [Python verification](evidence/final/independent-verifier.stdout.log).

## Five actual cycles

The same context was retained. Each cycle used fresh input and current evidence; previous approval did not grant current authority.

| Cycle | Status | Scalar | New effect | Total effects | iZ questions |
| --- | --- | --- | --- | --- | --- |
| 1 | HoldIncompleteLaw | 0 | No | 0 | Missing risk-limit clause |
| 2 | HoldMissingEvidence | 0 | No | 0 | Missing y evidence at x=0,1,2,3,4 |
| 3 | FailClosedCounterproof | 0 | No | 0 | Counterexamples at x=3 and x=4 |
| 4 | VerifiedAction | 1 | Yes | 1 | The competing candidate's two counterexamples remain |
| 5 | VerifiedAction | 1 | Yes | 2 | The competing candidate's two counterexamples remain |

Cycle 3's `4x+1` candidate has gain 45 and risk 17. Its larger gain cannot waive the risk limit 10. Cycle 4 admits `2x+1`, gain 25/risk 9; cycle 5 admits the new action `2x+2`, gain 30/risk 10. SHADOW's nine gates are individually visible in the JSON.

Each accepted payload used 40 logical bytes of process-owned RAM. Recorded write/readback/hash elapsed times were **2,900 ns** and **3,600 ns** in this one run. These values are observations, not a performance benchmark, total cycle time, physical DRAM latency, or energy measurement. `physical_energy_joules` is null. The whole five-cycle process took approximately 0.093 seconds in the final execution, as recorded in commands.json; no latency target is claimed.

Inputs, complete LGC, questions, gates, receipts and persisted records: [five-cycle evidence](evidence/final/demo/cycle-1.json), [cycle 2](evidence/final/demo/cycle-2.json), [cycle 3](evidence/final/demo/cycle-3.json), [cycle 4](evidence/final/demo/cycle-4.json), [cycle 5](evidence/final/demo/cycle-5.json), [journal](evidence/final/demo/memory.wal).

## Adversarial and lifecycle checks

Executed tests include:

- Altered QUANTUM gain with a rebuilt trace is rejected; altered hardware receipt loses the integrity seal and produces a question.
- Changing or reordering current LGC material yields scalar 0; rejected evidence is retained as untrusted material inside a fresh incident trace.
- Rebuilding checksums cannot turn an altered persisted assessment or transferred D-BAB role into a valid record. The decoder checks canonical structure as well as hashes.
- Missing evidence in a competing candidate does not veto a fully evidenced candidate; its question is retained.
- Zero/unfunded payload budgets cannot be replaced by a positive model result. Integer boundaries and overflow are tested.
- Eight copied FFI handles have one successful consumption. Thirty-two concurrent attempts have one winner. Wrong bindings/nonces do not consume the legitimate grant; another process-local gate cannot consume its handle.
- Same context/action/value replay across engine reopen creates no second effect. A conflicting value refuses commit, leaves memory unchanged and prevents an automatic next cycle in that engine.
- Five separate CLI processes reconstruct one context while retaining current verification and exact prior receipt references.
- Truncated/corrupted WALs, oversized inputs, duplicate fields and a competing writer are rejected.
- Output failure after commit preserves a known durable receipt and is classified as delivery failure. Uncertain processing failure is not described as rollback.

The FFI concurrency result is consumption uniqueness, while the WAL tests concern uniqueness of the capsule's **local recorded effect**. Neither establishes exactly-once effects in an external service.

## Red-to-green evidence

These failures occurred in the new capsule during development, not in an executed native ESS-MAI run:

1. [Hardware receipt red test](evidence/red-hardware-receipt.log): receipt corruption originally left `CandidateReady`; corrected to `FailClosedIntegrity`, no seal and an explicit question.
2. [Storage red tests](evidence/red-storage-replay.log): a negative report's internal LGC and a positive report's mathematics were insufficiently checked on replay; both are now validated.
3. [Rehashed-role red test](evidence/red-rehashed-role.log): changing a witness role while rebuilding checksums was originally accepted; replay now compares the exact canonical prefix and final gate/outcome transcript.

All three corrections are covered by the final debug/release suites. Earlier `evidence/validation/` and focused green logs are development records; **use `evidence/final/` for the final artifact**.

## Artifact identity and reproduction

Final release executable SHA-256:
`6cb52bf1be2b6d7f75154259fea2bcd1024482116b0fcee0bf7028e45a9c0757`

Cargo.lock SHA-256:
`00967e96595c1a68bf7c0a9b30edc1bcd4f7a08220f67a4ed5d0ded1cec8d3d5`

Final recorded WAL SHA-256:
`495b50e28fb56d402169bbed91b14e77e624762cdd06d5f764aacb329659f73c`

The source/evidence files are listed in `SHA256SUMS`. Mathematical selection is deterministic; measured timing and hence complete trace/receipt hashes can differ on a fresh execution. Do not replace a recorded hash with a new run's hash and present it as the same artifact.

```text
python tools/reproduce.py evidence/my-new-run --target-dir /path/to/separate/build-cache
python tools/verify.py evidence/final/demo --self-test --manifest SHA256SUMS
```

The reproduction directory must not exist. The runner uses pinned cached Cargo dependencies (`--locked --offline`), creates only its requested output/build directories and temporary test files, and exits on a failed check. For a first clean machine without the dependencies, use `cargo fetch --locked` with network access before the offline run.

## Limits established by this result

This experiment establishes the listed behavior on one Windows host, inside one finite model and a local trusted process/storage boundary. It does not demonstrate native KiS/KiC end-to-end processing, a separately compiled external C client, a VERITAS integration, hostile-host resistance, authentication from SHA-256, general mathematical/scientific truth, complete-prefix rollback detection, physical power-loss recovery, or real energy/thermal measurement. The original ESS-MAI and AIGovern source files were read and hashed, not modified by this capsule work.

The next collaboration step is an agreed external interface and independently observed target effect. [The Takeshi note](TAKESHI_REVIEW_NOTE.txt) proposes a technical discussion accompanying this capsule. Its inclusion does not establish delivery to or agreement, endorsement or completed integration by its addressee; the full private runtime remains excluded.
