# POC011 mathematical and execution contract

This document specifies the bounded experiment implemented by this capsule. It does not replace the native ESS-MAI receipt protocol or claim a proof of the complete architecture. Executed results belong in [RESULTS.md](RESULTS.md).

## 1. Case identity and evidence

Let `D = {0,1,2,3,4}` and let a candidate `c` contain an identifier, integer coefficients `(a,b)` and samples `E_c = {(x,y)}`. Input validation rejects duplicate sample coordinates, duplicate candidate identifiers/models, unknown fields, out-of-domain coordinates and values outside the declared bounds.

The candidate function is:

```text
f_c(x) = a*x + b,  x in D
|a| <= 1,000,000; |b| <= 1,000,000
therefore |f_c(x)| <= 5,000,000
```

Only five domain points are considered. The implementation uses bounded integer arithmetic and enables overflow checks in release. There is no extrapolation to real-world inputs or other domains.

Context identity and candidate identity are separate linked objects:

```text
H_context = H("POC011_CONTEXT_V1", context_text, scope)
H_candidate = H("POC011_CANDIDATE_V1", canonical candidate encoding)
H_input = H("POC011_INPUT_V1", canonical complete input encoding)
```

`H` is SHA-256 over a domain and individually length-prefixed parts. Internal JSON encodings use typed, bounded structures and deterministic field order. Journal replay requires the exact canonical encoding. Candidate assessments carry the associated context hash; a candidate hash alone is not a scope grant.

## 2. Missing evidence, counterexamples and selection

```text
Missing(c) = D minus the coordinates supplied in E_c
Mismatch(c) = {(x,y) in E_c : y != f_c(x)}
Risk(c) = max over x in D of |f_c(x)|
Gain(c) = sum over x in D of f_c(x)
Violation(c,L) = {x in D : |f_c(x)| > L.risk_limit}
```

The applicable law must supply jurisdiction, risk limit, logical byte budget and elapsed-time budget. Absence of a required law field produces `HoldIncompleteLaw`, not an invented default.

With a complete law, a candidate is eligible exactly when its samples cover `D`, match the model, and contain no risk-limit violation. A mismatch or a limit violation produces an explicit bounded counterexample. Missing evidence without a counterexample produces `MissingEvidence`, not refutation. If no eligible candidate exists, the aggregate result distinguishes missing evidence, no candidates and counterproof. The runtime currently names pending evidence `HoldMissingEvidence`.

```text
Eligible(c,L) = [Missing(c) is empty]
                AND [Mismatch(c) is empty]
                AND [Violation(c,L) is empty]

selected = arg min over eligible c of
           (-Gain(c), Risk(c), H_candidate(c))
```

This order selects the largest gain, then smallest risk, then smallest hash. Hash ordering is a deterministic tie-breaker, not a judgment of scientific quality. All hard conditions are applied before selection. Missing or rejected competing candidates remain visible but cannot waive a requirement for the selected candidate.

## 3. AI Authority paradigm, D-BAB and 0-NOI

In Bledar Gjata's proposed ESS-MAI paradigm, an authority is independent executive competence organised under constitutional laws. Its duty and jurisdiction define the work it owns. The following tuple makes that meaning explicit; it is an architectural description, not a claim that this capsule implements a generic constitutional engine:

```text
A = (Duty_A, Competence_A, Jurisdiction_A, ApplicableLaw_A)
Exercise(A, operation, case) requires
    operation in Competence_A
    AND (operation, case) in Jurisdiction_A
    AND the applicable law's conditions and required evidence.
```

LIGHT owns coordination, QUANTUM owns elimination/proposal, and SHADOW owns verification. The capsule's private stage carriers, fixed order, scope checks and current-case verification enforce the bounded sequence. A carrier transfers presented facts, not the preceding authority's competence. SHADOW's recomputation is its own verification work. The full GCL/Supreme hierarchy and the paradigm's general novelty are outside this finite experiment's proof.

The execution order is coordination, reasoning and current-case verification. D-BAB records an owner's preconditions before that owner's operation and records the actual observed outcome. D-BAB does not decide another owner's result. 0-NOI supplies the neutral vector:

```text
(state, knowledge_creation, verdict, authority, persistence, trigger)
    = (0,0,0,0,0,1)
```

The calling code supplies the actual control flow; a neutral pulse does not by itself execute an operation or satisfy a contract. A missing precondition cannot be repaired by issuing more pulses.

## 4. LGC, current verification and seal

Each LGC step binds its sequence, role, operation, payload and predecessor hash. The seed binds current input, context and the prior durable cycle receipt. The verifier checks the ordered chain and compares the current presented material with the expected transcript and recomputed assessment. Rehashing an altered gain therefore does not make that gain valid.

The final scalar is a hard conjunction of the required gates:

```text
Spine = scope AND complete_law AND exact_LGC AND current_evidence
        AND authority_order AND hardware_readback AND quantum_completion

Matrix = current selected candidate exists
         AND its missing/counterexample sets are empty
         AND candidate assessments retain the current context

V = Spine AND Matrix AND one_use_local_FFI_consumption
verification = 1 if V, otherwise 0
```

Spine precedes Matrix. No later presentation repairs an earlier failed gate, and no average of good gates masks a bad gate. SHADOW recomputes the current finite claim; historical references are only retrieval/context material. A preceding positive record is not a prerequisite-free approval of a later cycle.

The `500` seal is a domain-separated SHA-256 binding of the trace tip and current input, associated with the inherited seal concept. A refusal with intact evidence can retain an integrity seal; an integrity failure cannot retain one. A seal is not itself a positive verification bit, digital signature or proof of an issuer's authority.

## 5. Single-use local boundary

For an issued handle, native consumption uses atomic `compare_exchange(true,false)` on the owner's private slot. All copies address the same slot. Before consumption, the wrapper requires exact equality with the originally issued binding bytes and the source gate checks the expected nonce.

```text
successful_consumptions(slot) <= 1
issued_actions_per_gate <= 1024
1 <= binding_length <= 4096
```

A checked monotonic process-local nonce separates gate instances; an exhausted counter refuses issuance. The token stays private and the wrapper returns only success/error. This is a local implementation property, not a security proof under arbitrary memory writes, host compromise or process restart. Retaining the gate across cycles preserves already-consumed slots. Copying payload data or a handle cannot create another successful admission through that slot. This operational safeguard protects a particular action; it neither defines nor delegates the AI Authority's independent executive competence.

## 6. Process-memory execution receipt

The selected five signed integer values are encoded as little-endian 64-bit values:

```text
B_required = 8 * number_of_values = 40 bytes for this domain
n = min(floor(B_budget / B_required), 4)
execution requires n >= 1 and time_budget > 0
```

The implementation allocates process memory, writes the bytes, reads them back, checks their values and hashes the payload. It records `Instant` elapsed time. SHADOW requires byte equality, the expected hash, `B_actual = B_required <= B_budget`, and elapsed time no greater than the specified budget.

The byte budget counts logical payload bytes, not total allocator use, physical RAM pages or whole-process memory. The finite-model risk is unrelated to RAM safety. Time measurement does not establish a hard scheduling bound. No physical energy value is fabricated: the energy field remains absent.

## 7. Commit, effects and restart

The journal stores each cycle report and its optional new effect in one framed record:

```text
receipt_t = H("POC011_WAL_FRAME_V1", receipt_(t-1), length_t, body_t)
frame_t = length_u32_le || receipt_ascii_hex || body_t
```

Only verification `1` can produce an effect. The effect key is `(context_hash, action_id)`. An identical prior effect yields no additional effect; a different effect under that same key is a conflict. This local effect model does not provide distributed exactly-once execution.

The successful lifecycle is:

```text
current verification -> report/effect frame -> write -> flush -> sync
-> exact readback -> publish in-memory state -> release transient in-flight state
-> return output + questions -> CLI delivery
```

On unresolved execution/storage failure, successful completion/reset is not reported. A delivery failure after commit is reported separately: it does not roll back the durable record or imply that no effect occurred. An owner reopening the store validates the complete journal before continuing; it cannot infer an external side effect from a partial frame. A complete synchronized record can be recovered even if its writer did not deliver the final acknowledgement.

Restart is this capsule's explicit lifecycle adaptation. It does not erase memory, rewrite negative evidence or reset the FFI registry within a running engine. Full-prefix rollback remains outside the guarantee without an external trusted head. Physical power-loss protection is not inferred from local sync/readback tests.

## 8. Reproducible expected sequence

For the built-in five inputs, the expected verification sequence is `(0,0,0,1,1)` and cumulative effect counts are `(0,0,0,1,2)`. Each component is a separate scalar cycle result, not the native two-bit verdict. These expected values follow from the specific evidence fixtures and execution conditions; adverse hardware or storage outcomes must remain failures, not be overridden to match the sequence.
