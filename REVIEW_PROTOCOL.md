# Reproduce and challenge POC011

Read README.md, EXPLAINED.md, SOURCE_MAP.md and RESULTS.md before interpreting the result. Report the exact commit, operating system, Rust version, command, input, output and expected versus actual behavior. Criticism and negative findings are welcome; identify AI assistance if used, without treating it as a substitute for evidence.

1. Verify SHA256SUMS against the checked-out release. Fetch locked dependencies once if absent, then run the documented offline commands in a new output directory.
2. Reproduce five current-evidence cycles. Check that the first three create no admitted effect and the last two create distinct effects. Earlier gaps and counterexamples must remain visible.
3. Try a missing sample, a contradiction, an out-of-domain sample, a hard-risk breach and arithmetic boundary values. No good score may compensate for a mandatory failure.
4. Mutate a proposal or its trace. Rebuilding checksums must not replace the required semantic checks. State precisely whether the adversary controls an input, persisted bytes or the entire trusted process.
5. Exercise handle copies and concurrency using the included tests. Distinguish one-use admission from authority identity, and from effects in an external system.
6. Restart separate CLI processes. Distinguish local duplicate-effect suppression, incomplete-frame refusal and the explicitly unsupported complete-prefix rollback attack.
7. Review the proposed competence boundaries: coordination, proposal and verification are different duties. Identify an exact duplicated execution or unauthorized crossing, rather than inferring one from multiple function calls alone.

A successful reproduction supports only the exercised property. A counterexample should narrow or invalidate the affected claim and remain available for re-examination. A code-hosting page, DOI, archive, index entry or review request is not an endorsement. Public reports can be filed in this repository's Issues; substantive contributions should retain their authorship.
