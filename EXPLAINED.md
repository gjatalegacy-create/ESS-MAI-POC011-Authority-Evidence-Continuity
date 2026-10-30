# POC011 explained: distinct competences, one evidence-bound flow

**ESS-MAI POC011: Authority, Evidence and Durable Continuity** is a runnable research capsule by Bledar Gjata / Gjata Legacy. It puts a narrowly stated question before reviewers: can a sequence of different executive competences retain gaps and counterexamples, reject unsupported actions, and admit an evidenced action without duplicating its local recorded effect?

## The proposed AI Authority paradigm

In the ESS-MAI proposal, an AI Authority is a computational entity organised around a defined duty, competence, jurisdiction and governing law. It performs its own work, such as coordination, reasoning or verification. Permission or refusal is a consequence of that work; a permission token alone is not the entity. Independence is bounded by the applicable law and jurisdiction. Hierarchy does not give a superior every subordinate's competence.

GCL is the constitutional order, not another executive authority or a voter. ESS-MAI's intended internal organisation is GCL, Supreme, laws and hierarchical authorities. An objective-seeking agent would be an external participant in a future integration. No agent or VERITAS integration is implemented here.

Several calls to an authority can fulfil different competences. Recording an input, coordinating its candidates and handing over the result are different duties. Likewise, QUANTUM's proposal and SHADOW's independent verification are different responsibilities even when they inspect the same mathematical values. Counting function calls does not establish duplicated authority.

The author's clarified discipline concerns the same concrete execution within a cycle: performing another competence is not repeating that execution. A new cycle may legitimately exercise the competence again, with current checks. POC011 does not impose a blanket prohibition on processing unchanged content in later cycles.

This capsule implements bounded role separation, current-evidence verification, process-local single-use admission and local effect continuity. It does not extract the entire native authority lifecycle or establish every property of a persistent AI Authority entity. The proposed paradigm and the particular tested mechanisms must be evaluated separately.

## What happens, in order

1. LIGHT binds the current input, context and candidate identities, and retrieves bounded references to earlier records in that context.
2. QUANTUM computes the declared finite model, records missing samples and counterexamples, eliminates inadmissible candidates and selects from those meeting all conditions.
3. D-BAB witnesses the relevant preconditions and actual outcome. It does not replace the actor, authorize it or decide another competence's result. 0-NOI supplies a neutral borrowed trigger; the caller retains the checks.
4. SHADOW checks the current proposal, trace and RAM receipt. Spine checks precede Matrix checks. The final bit is the conjunction of mandatory gates, not a vote or a compensating average.
5. A successful local admission consumes one handle slot. The report and optional local effect are committed together, flushed, synchronized and read back. Only then is new memory state made visible, transient processing released, and output plus questions returned.

`EXECUTE` in this experiment means actual finite-model computation and a locally recorded admitted effect. It is not permission to operate an external robot or service. The RAM payload write/readback is real process-owned memory work; it does not establish physical energy, thermal safety or hard real-time guarantees.

## Why three initial zeros are useful

| Cycle | Evidence currently available | Outcome | What persists |
|---|---|---|---|
| 1 | No risk-limit clause | `HoldIncompleteLaw`, bit 0 | Missing clause and exact question; no effect |
| 2 | Limit 10, but no required samples | `HoldMissingEvidence`, bit 0 | Missing observations; no effect |
| 3 | Samples for `4x+1`, risk 17 | `FailClosedCounterproof`, bit 0 | Counterexamples at x=3 and x=4; no effect |
| 4 | Complete evidence for `2x+1`, risk 9 | `VerifiedAction`, bit 1 | Accepted local effect plus retained competitor objections |
| 5 | New action `2x+2`, risk 10 | `VerifiedAction`, bit 1 | Second distinct local effect; fresh checks and retained history |

These are the implemented POC011 status names. In particular, its pending-evidence HOLD must not be silently relabelled as every native authority's failure semantics. A missing sample is knowledge of a gap; an observed contradiction is counterevidence. They remain distinct. Success in cycle 4 neither erases the earlier failure nor grants authority for cycle 5.

For x in {0,1,2,3,4}, the model is y(x)=a*x+b. Gain is the sum of model values and risk is their maximum absolute value. Selection orders eligible candidates by gain descending, risk ascending and candidate SHA-256 ascending. The model and samples are experimental fixtures, not independent empirical observations. No conclusion beyond that finite domain follows.

## What is extracted and what is adapted

The selected Light single-use gate code and the later neutral 0-NOI primitive are exact extractions with source and extraction hashes. The finite model, bounded authority orchestration, witness integration, trace checks, storage frame and restart lifecycle are documented adaptations. [SOURCE_MAP.md](SOURCE_MAP.md) and the provenance manifests identify them individually. The private AIGovern-Cyber runtime is not included.

The recorded tests cover 38 Rust tests in debug and again in release, a five-cycle run and 17 independent-verifier self-tests. Two configurations do not mean 76 different properties. [RESULTS.md](RESULTS.md) gives the logs, commands and failure cases. Automated CI, if completed, is an additional execution environment, not independent peer review.

## Contribution and open invitation

State machines, scoped capabilities, atomic consumption, hashing, journals, mathematical checking and replay protection are established techniques. The author proposes the ESS-MAI constitutional organisation of independent executive competences and its use as a governing core or integration component. POC011 supplies a bounded, reproducible materialization for criticism; it does not establish historical novelty, patentability or general architectural completeness.

The included code and documentation are offered freely under Apache-2.0. Reproduction, adaptation, disagreement and attributed technical contributions are welcome. No review, endorsement or ownership transfer is required to download this capsule. Future application as a core in other systems is a research direction, not an integration already demonstrated here.

In Albanian: **i njëjti autoritet mund të kryejë kompetenca të ndryshme; kjo nuk është ribërje e së njëjtës detyrë.** POC011 demonstron një rrjedhë të kufizuar me evidencë, ndalime dhe ekzekutim lokal. Paradigma AUTORITETI AI paraqitet shprehimisht si propozim i Bledar Gjatës / ESS-MAI për shqyrtim dhe përdorim të lirë sipas licencës së përfshirë.

## Known boundary

The host, program and storage are trusted. Hashes are not authentication. A valid old journal prefix can be substituted without detection if the entire storage is controlled. No external exactly-once effect, native paired verdict, complete native KiS/KiC pipeline, independent scientific validation or complete authority hierarchy is claimed. Specific negative results are part of the contribution, not erased to obtain a positive outcome.
