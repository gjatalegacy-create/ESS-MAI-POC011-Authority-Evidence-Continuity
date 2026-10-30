---
license: apache-2.0
language:
  - en
tags:
  - ess-mai
  - rust
  - reproducibility
  - governance-first
  - software-verification
  - authority
pretty_name: "ESS-MAI POC011: Authority, Evidence and Durable Continuity"
---

# POC011 — Authority, Evidence and Durable Continuity

[Software DOI](https://doi.org/10.5281/zenodo.23809527) · [Companion preprint](https://doi.org/10.5281/zenodo.23809644) · [GitHub](https://github.com/gjatalegacy-create/ESS-MAI-POC011-Authority-Evidence-Continuity) · [Hugging Face](https://huggingface.co/datasets/gjata-legacy/ess-mai-poc-011-authority-evidence-continuity) · [All public links and indexing limits](PUBLIC_LINKS.md)

Start with [the precise explanation of the paradigm and this experiment](EXPLAINED.md), then [the reproduction and critique protocol](REVIEW_PROTOCOL.md). Several calls may perform different competences; they must not be counted as repeated authority solely because the same role appears again.

POC011 materializes a bounded part of Bledar Gjata's proposed **AI Authority paradigm** in ESS-MAI. An authority is an independent executive competence: a defined duty, the competence to fulfil it, a jurisdiction and governing laws within a constitutional hierarchy. Its work can be coordination, reasoning or verification; allowing or stopping an action is a consequence of applying its law. Independence means exercising its own competence within that jurisdiction.

ESS-MAI's intended constitutional organisation comprises GCL, Supreme, laws and hierarchical authorities. GCL supplies the constitutional order through the applicable laws and principal authorities, rather than a new global decision-maker in every operation. POC011 implements a limited sequence drawn from that method: LIGHT coordinates a scoped request, QUANTUM eliminates unsupported or inadmissible candidates, and SHADOW independently checks the current proposal before a local effect can be committed. Five successive inputs show how a missing law, missing evidence, a counterexample and corrected evidence lead to different outcomes without erasing the earlier record. The declared author contribution is this architectural organisation and its materialization; these tests do not establish the paradigm's general novelty or the entire hierarchy's correctness.

The output is a **scalar verification bit, `0` or `1`**. It concerns one action in one finite mathematical model. It is not the native ESS-MAI/AIGovern-Cyber `(0,0)` / `(1,1)` protocol, the full constitutional runtime, a security certification, or an integration with VERITAS or another external system.

Author and architecture: **Bledar Gjata / Gjata Legacy**. ESS-MAI public source: [gjatalegacy-create/ESS-MAI](https://github.com/gjatalegacy-create/ESS-MAI). Author identifier: [ORCID 0009-0003-7854-266X](https://orcid.org/0009-0003-7854-266X).

## What runs

The current branch includes a [memory-reference integrity correction](MEMORY_INTEGRITY_FIX.md): Rust now checks historical references against the actual prior records before commit and on reopen. The original `v0.1.0` archive remains unchanged and contains the earlier gap. This correction preserves current-case SHADOW verification and the five-cycle fixture; it does not add history authentication or remove the documented rollback limitation.

The declared domain is `x = 0,1,2,3,4`. A candidate supplies `a`, `b` and evidence samples for `y = a*x + b`. QUANTUM computes every model value and checks the supplied samples. Complete, matching samples and compliance with the hard risk limit are necessary for eligibility. Missing samples and observed contradictions remain distinct.

Among eligible candidates, selection orders gain descending, risk ascending and candidate SHA-256 ascending. Here, gain is the sum of the five values and risk is their maximum absolute value. These are deliberately small model quantities, not probabilities or physical safety measurements. A larger gain cannot buy an exception to a hard constraint.

The selected values are actually encoded into process-owned RAM, read back and hashed. SHADOW checks the resulting receipt, recomputes the candidate assessment and selection, verifies the trace and context, then conjoins its gates. A local one-use capability is consumed only after those checks. The journal records an admitted effect only for verification `1`.

| Declared authority or supporting mechanism | Responsibility in this capsule |
| --- | --- |
| LIGHT | Bind input/context, coordinate candidate identities and present bounded references to prior records in the same context. |
| QUANTUM | Recompute the finite model, retain gaps/counterexamples, eliminate invalid candidates and propose an eligible candidate. |
| D-BAB | Record the actual preconditions before the corresponding operation and the observed outcome afterward. It neither grants permission nor vetoes another role. |
| 0-NOI | Supply a neutral borrowed trigger. The caller performs the operation and retains its checks. |
| SHADOW | Apply Spine checks, then Matrix presentation/checks, then the final conjunction and local single-use boundary. |
| Persistent store | Commit the report and optional local effect together, verify the written bytes, then make the new state visible. |

The three executive authorities do not vote or inherit one another's competence. D-BAB, 0-NOI and persistence support their operation without becoming additional decision-making authorities. SHADOW's recomputation fulfils its own verification duty; it does not take over QUANTUM's responsibility for proposing an action. A compulsory failure cannot be hidden in an average. An incomplete candidate does not automatically invalidate a different candidate that satisfies every applicable requirement.

## The five-cycle fixture

These fixture outcomes were reproduced in the final execution in [RESULTS.md](RESULTS.md). They are not a rule that repetition earns approval.

| Cycle | Current input | Expected status | Verification | Cumulative durable effects |
| --- | --- | --- | --- | --- |
| 1 | The risk limit is absent. | `HoldIncompleteLaw` | 0 | 0 |
| 2 | Risk limit 10 is present; required samples are absent. | `HoldMissingEvidence` | 0 | 0 |
| 3 | Full samples for `y=4x+1`; maximum absolute value 17 exceeds 10. | `FailClosedCounterproof` | 0 | 0 |
| 4 | `y=2x+1` has complete samples, gain 25 and risk 9; the inadmissible competitor is retained. | `VerifiedAction` | 1 | 1 |
| 5 | A new action proposes `y=2x+2`, gain 30 and risk 10, with fresh complete samples and the same invalid competitor. | `VerifiedAction` | 1 | 2 |

Cycles 4 and 5 retain questions about the eliminated competing candidate. A successful selected action does not delete contradictory material. Conversely, a gap alone is not a scientific refutation: it states what evidence has not been supplied.

The examples are deterministic mathematical fixtures. Their samples are supplied for this bounded experiment, not independent empirical measurements. Nothing is established outside the declared five-point domain.

## Reproduce locally

Use **Rust 1.98.0**, the exact toolchain of the recorded validation and the pinned hosted reproduction workflow. Rust 1.99.1 makes the existing `fetch_update` call a deprecation error under this package's unchanged `warnings = "deny"` policy; newer-toolchain compatibility is not claimed. See [the CI compatibility erratum](CI_STATUS.md). `Cargo.lock` pins the dependencies. Offline commands require those pinned crates to exist in the local Cargo cache; they do not silently fetch dependencies.

Run from this directory:

```text
cargo test --locked --offline
cargo test --locked --offline --release
cargo run --locked --offline --release -- demo demo-run
cargo run --locked --offline --release -- inspect demo-run/memory.wal
```

`demo-run` must not already exist. The demo creates that directory, writes five input JSON files, five cycle-output JSON files and `memory.wal`. It prints each output and the final inspection as JSON. Build/test commands create Cargo build artifacts. Tests use isolated temporary files. Inspection reads the journal and obtains a shared file lock; it does not admit a new action.

The CLI also accepts `init WAL` and `cycle WAL INPUT.json`. The latter opens an existing journal and requires the exact next durable cycle number. Invalid input or an unresolved execution/storage failure exits nonzero. Diagnostics distinguish command failure, processing failure with uncertain commit status, and delivery failure after a known durable commit. They do not invent rollback or authorize an automatic retry.

[RESULTS.md](RESULTS.md) records 38 passing Rust tests in both debug and release, the actual five cycles, and the independent Python verification with 17 self-tests. It also contains exact reproduction commands and known limits.

## Continuity and memory

Each cycle includes its current input/context hashes and the previous durable receipt. LIGHT retrieves at most eight references from earlier records with the same context. Those references expose earlier positive/negative candidate identities and question counts; they do not authorize the new action. The user still supplies the current candidates and evidence.

SHADOW verifies the current proposal. It does not repeat earlier scientific judgments to manufacture a new verdict. Opening a journal does validate its stored structure and integrity before reconstruction; this storage replay is a different task from adjudicating current evidence. This capsule uses a bounded full replay on reopen, not a new low-latency indexing claim.

Reports with bit `0` are also durable records, with no admitted effect. After a complete frame has been written, flushed, synchronized and read back identically, the engine publishes its new memory state, releases transient in-flight state and returns the cycle output plus its exact gap/counterexample questions. The CLI then delivers that output; delivery failure cannot undo the commit and must not be confused with refusal. The journal, evidence and live FFI registry are retained. This lifecycle is a capsule adaptation; canonical GCL's phase contract is not itself this persistent state machine.

An effect is a local record of model values, keyed by context and action identity. Repeated identical effects are not applied twice; a conflicting effect for the same key is refused. This is not a transaction over a remote service, robot, network or bank account.

## Provenance and boundaries

- [SOURCE_MAP.md](SOURCE_MAP.md) identifies inspected ESS-MAI and later AIGovern methods and distinguishes adaptations from exact extraction.
- [provenance/ffi_zero_noi.json](provenance/ffi_zero_noi.json) records exact source hashes, inclusive line ranges and extraction hashes.
- `src/source_ffi.rs` preserves the selected canonical Light token, handle and atomic slot code byte-for-byte. The wrapper adds bounded issuance, exact binding checks and process-local separation between gate instances.
- `src/zero_noi.rs` is an exact copy of the later AIGovern neutral primitive. It is not presented as original v1.8.9 code.
- The finite model, current-case trace verifier, D-BAB event integration, persistence frame and post-commit restart are explicit capsule adaptations. The familiar mathematics, SHA-256 and compare-and-swap are not claimed as newly invented algorithms.

"No copied authority" has a precise limited meaning here: copied handle bytes reference one consumable slot, and only one consumption succeeds. This slot protects a particular action's admission; the handle neither defines nor transfers the AI Authority's independent executive competence. It does **not** mean that this entire program never copies data. The internal `extern "C"` probe exercises the C calling convention with copied handles; it is not a separately compiled C client or a public external integration interface. That interface and its threat model remain future integration work.

Hashes, LGC and the `500` seal bind declared content and trace order. They are not signatures or proof of the identity of a trusted issuer. The capsule assumes its own code and process boundary have not been replaced or arbitrarily corrupted. It does not protect against a hostile OS or arbitrary unsafe writes into the owning process.

The RAM receipt measures a logical payload write/readback and elapsed time. It does not measure DRAM energy, physical page residence, CPU temperature or a sensor reading. Its time limit is checked cooperatively, not enforced as a hard real-time deadline. `physical_energy_joules` remains absent.

The journal rejects incomplete frames and integrity violations and uses exclusive writer ownership. A complete old valid prefix can still be substituted without detection if an attacker controls the entire journal: no external trusted head or monotonic anchor exists. Successful `sync_all` and readback are the declared local durability boundary, not a proof against every device failure. Physical power-loss behavior is not established by this experiment; any process-interruption tests must be read within their recorded scope in RESULTS.md.

## Review and working method

AI-assisted implementation was used under the author's architecture and direction. The working method combines human source review with reproducible checks; the checks actually completed for this capsule are recorded in RESULTS.md. Author review before publication remains a separate responsibility, and these checks are not independent peer review. Reviewers are invited to challenge the exact admission boundary, trace semantics, capability lifetime and commit ordering, and to provide reproducible counterexamples. [TAKESHI_REVIEW_NOTE.txt](TAKESHI_REVIEW_NOTE.txt) proposes a bounded technical discussion without requesting access to another project's private source.

This capsule is distributed under Apache-2.0; see `LICENSE` and [NOTICE](NOTICE). That license applies to the materials included here. It does not publish or relicense the entire AIGovern-Cyber system or any excluded private implementation.
