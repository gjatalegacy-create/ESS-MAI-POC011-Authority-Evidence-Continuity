# POC011 source and method map

This capsule is a new, bounded adaptation of ESS-MAI methods. This document records source inspection and the experiment's derivation. Executed checks and their limits are recorded separately in [RESULTS.md](RESULTS.md).

## Source identities

`C` is canonical ESS-MAI v1.8.9 phase 015, under `v1.8.9_ess_mai_phase_015-ws/v1.8.9_ess_mai_phase_015/ess_mai`. `A` is the current AIGovern-Cyber derivative inspected at `Desktop/aig-c/codex_zvn/runtime`. The machine-readable [methodology manifest](provenance/methodology.json) records each **whole-file SHA-256**, byte size, logical line count and cited 1-based inclusive ranges. Hashes cover original raw bytes, including line endings. The selected Light gate blocks and the small derivative 0-NOI primitive are included explicitly; the rest of the private runtime is excluded.

D-BAB and 0-NOI must not be described as literal extractions from canonical v1.8.9: the bounded Rust/Cargo search found no such named mechanisms there. The later materialization files explicitly identify themselves as new POC-native work. The capsule follows the documented method and identifies these later sources separately.

## Audited blocks

| ID | Source | Lines | Mechanism |
| --- | --- | --- | --- |
| C_GCL | `C:gcl-constitution/src/constitution.rs` | 7-40, 45-69, 300-328, 372-395 | Phase jurisdiction and three executive roles |
| C_PHASE | `C:gcl-constitution/src/phase.rs` | 1-20 | Coordination, reasoning, verification phases |
| C_LIGHT | `C:light/src/light_coordinator.rs` | 5-20, 378-391, 439-445 | LIGHT coordination and dispatch prerequisite checks |
| C_QUANTUM | `C:quantum/src/orchestrator.rs` | 7-15, 129-147, 191-240, 312-318 | QUANTUM elimination, proposal and ephemeral reset |
| C_PD_LIGHT | `C:light/src/pd_light.rs` | 1-8, 64-78 | Verified output plus iZ delivery |
| C_SPINE_ORDER | `C:shadow/src/shadow_gateway.rs` | 460-511 | Spine before Matrix/pipeline |
| C_SPINE | `C:shadow/src/shadow_spine.rs` | 170-190, 214-247 | Multi-verification attestation and conjunction |
| C_PIPELINE | `C:shadow/src/shadow_pipeline.rs` | 87-179 | Router, Matrix, route checks, Judiciary and final boundary |
| C_MATRIX | `C:shadow/src/shadow_matrix.rs` | 28-37, 43-54, 82-95 | Historical Matrix dimensions and distinct bit |
| A_MATRIX | `A:shadow/src/shadow_matrix.rs` | 46-94, 126-141 | Current-case Matrix projection |
| A_NBES | `A:shadow/src/shadow_gateway.rs` | 1230-1308 | NBES and currently presented KiE case |
| A_DBAB_WITNESS | `A:shadow-contracts/src/pd_failure_witness.rs` | 1-3, 95-109, 186-205 | Data-only witness of actual request and refusal |
| A_DBAB_NOT_REUSED | `A:shadow-contracts/src/materialization/dbab.rs` | 1-8, 175-191, 270-292 | Later POC-native binding operator |
| A_NOI_PRIMITIVE | `A:zero-noi/src/lib.rs` | 1-18 | Borrowed neutral pulse |
| A_NOI_CALLER | `A:zero-noi/caller.rs` | 1-3, 17-26, 62-74 | Caller-owned scoped delivery |
| A_NOI_LATER | `A:shadow-contracts/src/materialization/zero_noi.rs` | 1-20, 80-117 | Later evidenced trigger adapter |
| A_PD | `A:shadow-contracts/src/pd_memory.rs` | 23-65, 69-134, 137-193, 242-276, 298-368 | SHA256 context identity, witness classes and linked trace |
| A_PD_STORE | `A:shadow/src/pd_memory_store.rs` | 140-273 | Owner prepare/publish boundary |
| A_IZ_ORDER | `A:shadow/src/native_cycle.rs` | 183-200 | Prerequisites, durable T1, then final iZ |
| A_CAUSES | `A:shadow-contracts/src/iz_frontier.rs` | 31-79, 90-121 | Missing material versus counterproof versus unresolved cause |
| A_PAIR_BOUNDARY | `A:shadow-contracts/src/boundary/verification_receipt.rs` | 25-26, 39-71 | Native paired receipt boundary |
| A_HW_RECEIPT | `A:quantum/src/hw_real/runtime.rs` | 67-99, 306-312, 345-364 | Resource release accounting |
| A_HW_HOLD | `A:quantum/src/hcp_pro.rs` | 37-54, 172-185 | Hardware Hold means keep current territory |
| A_RESET | `A:quantum/src/orchestrator.rs` | 312-318 | Ephemeral cycle reset |

## What is preserved

ESS-MAI's proposed internal organisation is authorities-only: GCL, Supreme, laws and hierarchical authorities, with no agents. POC011 implements the declared bounded sequence below; an external agent belongs to a future agreed integration, not to the capsule's internal authority model.

LIGHT anchors and coordinates; QUANTUM computes, eliminates and proposes; SHADOW checks the current case. These are bounded materializations of the proposed ESS-MAI AI Authority paradigm: each exercises its own executive competence under its applicable law and within its jurisdiction. They do not vote on a conclusion or acquire one another's competence. In the audited native sequence, the gateway runs Spine before the Matrix-containing pipeline. Mandatory checks are conjoined, and later presentation cannot supply evidence retroactively to an earlier judgment. The capsule preserves that relative ordering without claiming to reproduce every native node or prove the paradigm's general novelty.

D-BAB witnesses preconditions and actual failure. It grants no authority and casts no veto. The appropriate source is `A_DBAB_WITNESS` and the `ConditionWitness` portion of `A_PD`. The later `A_DBAB_NOT_REUSED` binding operator constructs a `BoundObligation` and is described as authority-producing; copying it as a neutral witness would change the intended semantics. A completed-request witness does not establish what happened before the request existed.

0-NOI is the borrowed neutral pulse: trigger one, authority/verdict/knowledge/persistence zero. The caller retains all law and evidence checks and preserves the called operation's error. The richer later evidenced-trigger adapter is a separate materialization, not an additional authorization power.

The PD/LGC adaptation binds exact context, source input, ordered predecessors, questions and revisions with SHA-256. Construction, a matching hash or successful replay is not authorization, identity authentication, or evidence that a scientific claim is true. Persistent records remain available across cycles. The capsule releases transient in-flight state after durable commit and readback, before returning output for CLI delivery; a later delivery failure does not undo the commit.

## New mathematical experiment

The declared domain is the five integers `x = 0, 1, 2, 3, 4`. A candidate proposes `y(x) = a*x + b`. Its bounded utility is `gain = sum(y(x))`; its bounded risk is `risk = max(abs(y(x)))`. Surviving candidates are ordered by gain descending, risk ascending, then canonical candidate SHA-256 ascending. The hard risk limit and every required law, scope, competence and evidence check remain mandatory. High gain cannot compensate for a hard-limit failure. Arithmetic must be bounded and must not silently wrap.

This is **new integer-model utility**, not canonical ESS-MAI physics, confidence, a probability estimate, a hardware safety measurement, or a general scientific result. SHA-256 supplies deterministic ordering and integrity links, not new authority.

The output bit `0` or `1` means only whether the declared bounded action passed its required verification. It is deliberately **not** the native `(0,0)` / `(1,1)` receipt protocol or a replacement for native seals and knowledge admission.

## Five-cycle continuity

1. Incomplete law: HOLD with the exact missing condition and no authorized action.
2. Complete law but missing required evidence: `HoldMissingEvidence` blocks admission while D-BAB records the gap and iZ asks for it. This pending-evidence state is not a scientific counterexample.
3. Every offered candidate is eliminated: preserve the reasons and report no valid proposal.
4. A valid bounded action: independently verify and commit, then release transient state and return output.
5. A fresh cycle eliminates a worse proposal and can accept another valid action only after current evidence is checked again. Previous success does not waive the checks.

This sequence is an experimental fixture, not a promise that five turns produce success. Missing conditions, an authority refusal, transport uncertainty and a scientific counterproof remain distinct. A missing-evidence trace does not itself refute the thesis.

The capsule's restart means a new scoped cycle with retained evidence. Canonical `CollapsePhase` contains Coordination, Reasoning and Verification; it does not contain the capsule's durable interruption/restart state machine. That state machine is an explicit adaptation, not inherited code.

## Hardware and scope limits

The source `HardwareReleaseReceipt` records release accounting. In the inspected implementation, `completed` is set from `lease_released`, and buffer counts are previously registered counts. It is not, by itself, proof that an OS has physically reclaimed RAM or that a scientific action succeeded. Also, source `HardwareDirective::Hold` means retain the current resource territory; it must not be confused with incomplete-law HOLD.

The map does not certify the entire canonical or derivative system. It does not claim every historical branch is active, production readiness, scientific completeness, or successful capsule tests. Runtime evidence and an adversarial review are required before any claim of working behavior.
