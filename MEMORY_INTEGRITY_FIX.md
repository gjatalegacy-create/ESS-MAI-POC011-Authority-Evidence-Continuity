# Historical memory-reference integrity correction

The original `v0.1.0` runtime could accept a canonically encoded, rehashed journal whose LIGHT memory references disagreed with earlier records still present in that same journal. The independent Python verifier already rejected these inconsistencies. This was a runtime consistency gap, separate from the documented Rust 1.99.1 compiler compatibility issue.

An audit reproduced four changes to the last record: a wrong prior receipt, removed references, an invented negative-candidate hash and an incorrect question count. Regression testing also covers a changed positive-candidate hash. All dependent hashes were recomputed while the earlier records remained unchanged. Both `inspect` and reopening for another cycle accepted the old inconsistent history. Fresh missing evidence still produced scalar `0`; the audit did not demonstrate a bypass of current scientific evidence or external exactly-once execution.

## Correction and architectural boundary

The existing reference builder now lives in `storage::memory_references` and is shared by the Engine and the storage checks. For a normal report at cycle `t`, the required equality is:

```text
actual_memory_refs(t) = last_8_same_context_references(records_before_t)
```

Equality includes cycle, context, receipt, positive and negative candidate lists, question count and list order. Receipts are derived from the full journal before context filtering, preserving correctness for interleaved contexts. `Store::commit` checks the equality before writing or changing state; `decode` checks it before accepting the next record. A previously validated `FailClosedIntegrity` incident retains its separate zero-result incident trace, which deliberately has no trusted LIGHT coordination.

This is a storage-link check. SHADOW still verifies the current case without re-adjudicating historical science. LIGHT, QUANTUM, SHADOW, their order, finite-model formulas, thresholds, FFI, D-BAB and 0-NOI are unchanged. The on-disk format is unchanged, and valid old journals remain readable.

## Regression evidence

Before the production correction, five new rehashed-reference regressions failed; the mixed-context and genuine-incident controls passed (exit 101: 5 failed, 2 passed). An eighth regression checks direct commit refusal and unchanged WAL/head/records/effects.

The corrected source passed 46 Rust tests in both debug and release, with zero failures or ignored tests. The five-cycle demonstration retained scalar results `0,0,0,1,1` and two durable effects. The independent Python verifier and its 17 self-tests passed. See [the maintenance verification record](evidence/memory-reference-fix/verification.json) and [the regression source](src/storage/memory_reference_tests.rs).

Reproduce with Rust 1.98.0:

```text
cargo test --locked --offline --lib memory_reference_tests
python tools/reproduce.py new-memory-check --target-dir target
```

The reproduction command creates a new evidence directory and build artifacts; it does not modify existing journal fixtures. The dependencies must already be cached for offline Cargo commands.

## Publication and limits

This correction applies to the maintained branch. The original `v0.1.0` tag, ZIP, DOI archives and recorded results are preserved; they are not retroactively described as corrected. The maintained source has its own updated `SHA256SUMS`.

Rehashed contradictions with records already present are now rejected. SHA-256 still does not authenticate the history's author. Whole-history substitution or rollback to a complete older valid prefix remains outside the guarantee without a trusted external anchor. Rust 1.99.1 compatibility remains a separate open issue; the declared toolchain stays Rust 1.98.0.
