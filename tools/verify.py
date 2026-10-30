#!/usr/bin/env python3
"""Independent stdlib verifier for POC011's bounded five-cycle evidence.

Reads files only. It does not execute Rust, access original-runtime state, or
remeasure reported timing. Hash consistency is not issuer authentication.
"""

import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import struct
import sys


ROOT = Path(__file__).resolve().parents[1]
MAGIC = b"POC011\x00\x01"
SCOPE = "poc011/affine-admission"
DOMAIN = list(range(5))
BITS = [0, 0, 0, 1, 1]
MAX_FILE = 64 * 1024 * 1024
GATE_NAMES = [
    "Spine.scope", "Spine.law_complete", "Spine.lgc_exact",
    "Spine.current_evidence", "Spine.authority_order", "Spine.hardware_readback",
    "Spine.quantum_completion", "Matrix.current_dossier", "Judiciary.single_use_local_ffi",
]
FIELDS = {
    "Input": "cycle context scope action_id question law candidates",
    "Law": "jurisdiction risk_limit byte_budget time_budget_ns",
    "Candidate": "id a b samples",
    "Sample": "x y",
    "Assessment": "candidate_id candidate_sha256 context_sha256 status missing_x counterexamples values risk gain",
    "Report": "input status verification selected_candidate_sha256 assessments hardware gates seal_500 lgc output questions",
    "Gate": "name passed",
    "Lgc": "input_sha256 context_sha256 previous_cycle_sha256 steps",
    "Step": "sequence role operation payload previous_sha256 sha256",
    "Effect": "action_id context_sha256 candidate_sha256 values",
    "Record": "version sequence previous_receipt_sha256 report new_effect",
    "Commit": "sequence receipt_sha256 effect_count new_effect durable readback_verified",
    "CycleOutput": "report commit gcl_restart_after_durable_commit next_cycle",
    "HardwareReceipt": "payload_sha256 requested_bytes actual_bytes elapsed_ns byte_budget time_budget_ns verified_readback physical_energy_joules",
    "MemoryReference": "cycle context_sha256 receipt_sha256 negative_candidate_sha256 positive_candidate_sha256 question_count",
}


class Invalid(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Invalid(message)


def integer(value, low, high, label):
    require(type(value) is int and low <= value <= high, f"invalid integer: {label}")
    return value


def unique_object(pairs):
    out = {}
    for key, value in pairs:
        require(key not in out, f"duplicate JSON key: {key}")
        out[key] = value
    return out


def parse(data):
    def bad_number(value):
        raise Invalid(f"non-integer/nonfinite JSON number: {value}")
    return json.loads(data, object_pairs_hook=unique_object,
                      parse_float=bad_number, parse_constant=bad_number)


def load(path):
    require(path.stat().st_size <= MAX_FILE, f"file too large: {path.name}")
    return parse(path.read_bytes())


def encode(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def value_map(value):
    """serde_json::Value uses sorted object keys without preserve_order."""
    if isinstance(value, dict):
        return {key: value_map(value[key]) for key in sorted(value)}
    if isinstance(value, list):
        return [value_map(item) for item in value]
    return value


def same(left, right):
    # Python equates True with 1; JSON/Rust do not. Preserve that distinction.
    return encode(value_map(left)) == encode(value_map(right))


def canonical(obj, kind):
    keys = FIELDS[kind].split()
    require(type(obj) is dict and set(obj) == set(keys), f"unexpected {kind} fields")
    out = {key: obj[key] for key in keys}
    children = {
        "Input": {"law": "Law", "candidates": ["Candidate"]},
        "Candidate": {"samples": ["Sample"]},
        "Report": {"input": "Input", "assessments": ["Assessment"],
                   "gates": ["Gate"], "lgc": "Lgc"},
        "Lgc": {"steps": ["Step"]},
        "Record": {"report": "Report", "new_effect": "Effect"},
        "CycleOutput": {"report": "Report", "commit": "Commit"},
    }
    for field, child in children.get(kind, {}).items():
        if out[field] is None and field == "new_effect":
            continue
        if isinstance(child, list):
            require(type(out[field]) is list, f"expected list: {kind}.{field}")
            out[field] = [canonical(item, child[0]) for item in out[field]]
        else:
            out[field] = canonical(out[field], child)
    if kind == "Step":
        out["payload"] = value_map(out["payload"])
    if kind == "Report":
        out["hardware"] = value_map(out["hardware"])
    return out


def hash_parts(domain, *parts):
    digest = hashlib.sha256()
    for part in (domain.encode("utf-8"),) + parts:
        digest.update(struct.pack("<Q", len(part)))
        digest.update(part)
    return digest.hexdigest()


def hash_value(domain, value):
    return hash_parts(domain, encode(value))


def text(value, minimum, maximum, label):
    require(type(value) is str and minimum <= len(value.encode("utf-8")) <= maximum,
            f"invalid text: {label}")


def validate_input(raw):
    inp = canonical(raw, "Input")
    integer(inp["cycle"], 1, 1024, "cycle")
    require(inp["scope"] == SCOPE, "foreign scope")
    for field in ["context", "action_id", "question"]:
        text(inp[field], 1, 2048, field)
    law = inp["law"]
    require(law["jurisdiction"] in [None, SCOPE], "foreign law jurisdiction")
    for key, limit in [("risk_limit", 10_000_000), ("byte_budget", 65536),
                       ("time_budget_ns", 60_000_000_000)]:
        if law[key] is not None:
            integer(law[key], 0, limit, key)
    require(len(inp["candidates"]) <= 4, "too many candidates")
    ids, models = set(), set()
    for candidate in inp["candidates"]:
        text(candidate["id"], 1, 128, "candidate id")
        for key in ["a", "b"]:
            integer(candidate[key], -1_000_000, 1_000_000, key)
        model = (candidate["a"], candidate["b"])
        require(candidate["id"] not in ids and model not in models, "duplicate candidate")
        ids.add(candidate["id"])
        models.add(model)
        require(len(candidate["samples"]) <= 5, "too many samples")
        xs = set()
        for sample in candidate["samples"]:
            integer(sample["x"], 0, 4, "sample x")
            integer(sample["y"], -10_000_000, 10_000_000, "sample y")
            require(sample["x"] not in xs, "duplicate sample coordinate")
            xs.add(sample["x"])
    return inp


def context_hash(inp):
    return hash_parts("POC011_CONTEXT_V1", inp["context"].encode(), inp["scope"].encode())


def law_gaps(inp):
    return ["law." + key for key in FIELDS["Law"].split() if inp["law"][key] is None]


def assess(inp):
    out = []
    for candidate in inp["candidates"]:
        values = [candidate["a"] * x + candidate["b"] for x in DOMAIN]
        samples = {sample["x"]: sample["y"] for sample in candidate["samples"]}
        missing = [x for x in DOMAIN if x not in samples]
        negatives = [
            f"candidate {candidate['id']}: x={x}, supplied y={samples[x]}, model y={y}"
            for x, y in zip(DOMAIN, values) if x in samples and samples[x] != y
        ]
        limit = inp["law"]["risk_limit"]
        if limit is not None:
            negatives += [
                f"candidate {candidate['id']}: x={x}, |y|={abs(y)} > risk_limit={limit}"
                for x, y in zip(DOMAIN, values) if abs(y) > limit
            ]
        out.append(dict(zip(FIELDS["Assessment"].split(), [
            candidate["id"], hash_value("POC011_CANDIDATE_V1", candidate), context_hash(inp),
            "EliminatedCounterproof" if negatives else "MissingEvidence" if missing else "Eligible",
            missing, negatives, values, max(map(abs, values)), sum(values),
        ])))
    return out


def winner(assessments):
    eligible = [a for a in assessments if a["status"] == "Eligible"]
    return min(eligible, key=lambda a: (-a["gain"], a["risk"], a["candidate_sha256"]), default=None)


def verify_hardware(raw, inp, selected):
    if raw is None:
        return False
    require(selected is not None, "hardware receipt without an eligible candidate")
    receipt = canonical(raw, "HardwareReceipt")
    payload = b"".join(struct.pack("<q", value) for value in selected["values"])
    for key in ["requested_bytes", "actual_bytes", "elapsed_ns", "byte_budget", "time_budget_ns"]:
        integer(receipt[key], 0, (1 << 64) - 1, "hardware " + key)
    require(receipt["payload_sha256"] == hashlib.sha256(payload).hexdigest(), "hardware payload SHA256")
    require(receipt["requested_bytes"] == receipt["actual_bytes"] == len(payload), "hardware byte count")
    require(0 < len(payload) <= receipt["byte_budget"] <= 65536, "hardware byte bound")
    require(receipt["byte_budget"] == inp["law"]["byte_budget"], "hardware law budget mismatch")
    require(receipt["time_budget_ns"] == inp["law"]["time_budget_ns"] > 0, "hardware time law mismatch")
    require(receipt["elapsed_ns"] <= receipt["time_budget_ns"], "hardware deadline exceeded")
    require(receipt["verified_readback"] is True, "hardware readback was not verified")
    require(receipt["physical_energy_joules"] is None, "unmeasured physical energy claim")
    return True


def source_struct_order(name):
    source = (ROOT / "src" / "authority.rs").read_text(encoding="utf-8")
    match = re.search(r"struct\s+" + re.escape(name) + r"\s*\{(.*?)\n\}", source, re.S)
    require(match is not None, f"cannot read {name} serialization layout")
    return re.findall(r"^\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_][a-z_0-9]*)\s*:", match[1], re.M)


def verify_report(raw, previous, history):
    report = canonical(raw, "Report")
    inp = validate_input(report["input"])
    inp_sha = hash_value("POC011_INPUT_V1", inp)
    ctx = context_hash(inp)
    gaps = law_gaps(inp)
    assessments = [] if gaps else assess(inp)
    require(same(report["assessments"], assessments), "current affine proof/risk/gain mismatch")
    selected = winner(assessments)
    selected_sha = selected["candidate_sha256"] if selected else None
    require(report["selected_candidate_sha256"] == selected_sha, "deterministic collapse mismatch")
    lgc = report["lgc"]
    require(lgc["input_sha256"] == inp_sha and lgc["context_sha256"] == ctx
            and lgc["previous_cycle_sha256"] == previous, "LGC current-case identity")
    require(len(lgc["steps"]) == 13, "unexpected normal demonstration LGC step count")
    coordination = lgc["steps"][2]["payload"]
    require(type(coordination) is dict, "missing LIGHT coordination")
    expected_coord = {
        "input_sha256": inp_sha, "context_sha256": ctx, "previous_cycle_sha256": previous,
        "candidate_hashes": [hash_value("POC011_CANDIDATE_V1", c) for c in inp["candidates"]],
    }
    for key, value in expected_coord.items():
        require(coordination.get(key) == value, "LIGHT binding mismatch: " + key)
    order = source_struct_order("Coordination")
    require(set(coordination) == set(order), "coordination schema mismatch")
    if "memory_refs" in coordination:
        verify_memory_refs(coordination["memory_refs"], ctx, history, previous)
    require(set(order) <= set(expected_coord) | {"memory_refs"}, "unverified coordination field")
    coord_typed = {key: coordination[key] for key in order}
    if "memory_refs" in coord_typed:
        coord_typed["memory_refs"] = [canonical(ref, "MemoryReference") for ref in coord_typed["memory_refs"]]
    reasoning_payload = lgc["steps"][5]["payload"]
    require(type(reasoning_payload) is dict, "missing QUANTUM reasoning")
    require(set(reasoning_payload) == {"assessments", "status", "selected", "hardware", "hardware_failure"},
            "unexpected reasoning fields")
    failure = reasoning_payload["hardware_failure"]
    require(failure is None or (type(failure) is str and 0 < len(failure) <= 2048), "hardware failure shape")
    require(selected is not None or failure is None, "hardware failure without attempted candidate")
    hw_ok = verify_hardware(report["hardware"], inp, selected)
    require(not (hw_ok and failure), "simultaneous hardware success/failure")
    if gaps:
        status = "HoldIncompleteLaw"
    elif selected is not None:
        status = "CandidateReady" if hw_ok else "HoldHardwareEvidence"
        require(hw_ok or failure is not None, "missing hardware outcome")
    elif any(a["status"] == "MissingEvidence" for a in assessments):
        status = "HoldMissingEvidence"
    else:
        status = "FailClosedCounterproof" if assessments else "FailClosedNoCandidates"
    expected_reasoning = {"assessments": assessments, "status": status, "selected": selected_sha,
                          "hardware": report["hardware"], "hardware_failure": failure}
    require(same(reasoning_payload, expected_reasoning), "QUANTUM outcome mismatch")
    gates = [{"name": name, "passed": passed} for name, passed in zip(GATE_NAMES[:7],
             [True, not gaps, True, True, True, hw_ok and failure is None, status == "CandidateReady"])]
    spine = all(g["passed"] for g in gates)
    matrix = selected is not None
    gates.append({"name": GATE_NAMES[7], "passed": matrix})
    consumed = spine and matrix
    gates.append({"name": GATE_NAMES[8], "passed": consumed})
    bit = int(all(g["passed"] for g in gates))
    integer(report["verification"], 0, 1, "scalar verification (not a native pair)")
    require(report["verification"] == bit and same(report["gates"], gates), "SHADOW gate conjunction")
    final_status = "VerifiedAction" if bit else status
    require(report["status"] == final_status, "report status mismatch")

    tip = hash_parts("POC011_LGC_START_V1", inp_sha.encode(), ctx.encode(), previous.encode())
    expected_steps = []

    def append(role, operation, payload):
        nonlocal tip
        payload = value_map(payload)
        sequence = len(expected_steps)
        sha = hash_value("POC011_LGC_STEP_V1", [sequence, role, operation, payload, tip])
        expected_steps.append(dict(zip(FIELDS["Step"].split(), [sequence, role, operation, payload, tip, sha])))
        tip = sha

    def witness(owner, stage, evidence):
        append("D-BAB", stage, {"owner": owner, "evidence": evidence})

    def pulse(owner):
        append("0-NOI", "trigger", {"owner": owner, "input_sha256": inp_sha,
                                    "vector": [0, 0, 0, 0, 0, 1]})

    witness("LIGHT", "preconditions", {"input_sha256": inp_sha, "law_gaps": gaps})
    pulse("LIGHT")
    append("LIGHT", "coordinate", coordination)
    witness("QUANTUM", "preconditions", {"coordination_sha256": hash_value("POC011_COORDINATION_V1", coord_typed), "law_gaps": gaps})
    pulse("QUANTUM")
    append("QUANTUM", "eliminate", expected_reasoning)
    witness("QUANTUM", "outcome", {"status": status,
            "assessment_sha256": hash_value("POC011_ASSESSMENTS_V1", assessments), "hardware_failure": failure})
    witness("SHADOW", "preconditions", {"received_tip": tip})
    pulse("SHADOW")
    append("SHADOW", "spine", {"gates": gates[:7], "conjunction": spine})
    append("SHADOW", "matrix", {"selected": selected_sha, "passed": matrix,
            "negative_evidence_retained": sum(bool(a["counterexamples"]) for a in assessments)})
    append("SHADOW", "judiciary", {"verification": bit, "all_required": True, "ffi_consumed": consumed})
    witness("SHADOW", "outcome", {"status": final_status, "verification": bit})
    require(same(lgc["steps"], expected_steps), "LGC event content, order or hash-chain mismatch")
    require(report["seal_500"] == hash_value("POC011_SEAL_500_V1", [500, tip, inp_sha]), "seal-500 mismatch")
    questions = [f"What is the applicable value for {gap} in scope {SCOPE}?" for gap in gaps]
    for assessment in assessments:
        if assessment["missing_x"]:
            questions.append(f"Provide current y evidence for candidate {assessment['candidate_id']} at x={assessment['missing_x']} under y=a*x+b.")
        questions += [f"{ce}. What corrected candidate or evidence addresses this exact counterexample within the same law?"
                      for ce in assessment["counterexamples"]]
    if not assessments and not gaps:
        questions.append("Which candidate and complete model table should LIGHT coordinate for this context?")
    if failure:
        questions.append(f"Hardware execution lacks a valid receipt ({failure}); what sufficient measured execution budget can be provided?")
    output = (f"Verified bounded action: {selected_sha}. Scope: {SCOPE}. No claim beyond the finite model." if bit else
              f"{final_status}: no action admitted. Current evidence and its gaps remain explicit.")
    require(report["questions"] == questions and report["output"] == output, "iZ output/question mismatch")
    return report, selected


def verify_memory_refs(refs, context, history, previous):
    require(type(refs) is list, "memory_refs must be a list")
    expected = []
    for index, record in enumerate(history):
        report = record["report"]
        if context_hash(report["input"]) != context:
            continue
        receipt = history[index + 1]["previous_receipt_sha256"] if index + 1 < len(history) else previous
        expected.append(dict(zip(FIELDS["MemoryReference"].split(), [
            record["sequence"], context, receipt,
            [a["candidate_sha256"] for a in report["assessments"] if a["status"] == "EliminatedCounterproof"],
            [report["selected_candidate_sha256"]] if report["verification"] == 1 else [],
            len(report["questions"]),
        ])))
    for ref in refs:
        canonical(ref, "MemoryReference")
        integer(ref["cycle"], 1, 1024, "memory cycle")
        integer(ref["question_count"], 0, (1 << 64) - 1, "memory question count")
    require(same(refs, expected[-8:]), "memory references are not exact prior same-context evidence")


def decode_wal(data):
    require(len(data) <= MAX_FILE and data[:8] == MAGIC, "WAL header/size")
    at, previous = 8, hash_parts("POC011_WAL_GENESIS_V1", MAGIC)
    records, receipts, effects = [], [], {}
    while at < len(data):
        require(len(records) < 1024 and len(data) - at >= 68, "incomplete WAL frame")
        length_bytes = data[at:at + 4]
        length = struct.unpack("<I", length_bytes)[0]
        require(0 < length <= 256 * 1024 and at + 68 + length <= len(data), "WAL frame length")
        receipt = data[at + 4:at + 68].decode("ascii")
        body = data[at + 68:at + 68 + length]
        require(receipt == hash_parts("POC011_WAL_FRAME_V1", previous.encode(), length_bytes, body), "WAL frame SHA256")
        record = canonical(parse(body), "Record")
        require(encode(record) == body, "noncanonical serde field order or encoding")
        sequence = len(records) + 1
        require(type(record["version"]) is int and record["version"] == 1
                and type(record["sequence"]) is int and record["sequence"] == sequence
                and record["previous_receipt_sha256"] == previous, "WAL sequence/predecessor")
        report, selected = verify_report(record["report"], previous, records)
        require(report["input"]["cycle"] == sequence, "input/WAL cycle mismatch")
        expected_effect = None
        if report["verification"] == 1:
            effect = dict(zip(FIELDS["Effect"].split(), [report["input"]["action_id"],
                context_hash(report["input"]), selected["candidate_sha256"], selected["values"]]))
            key = (effect["context_sha256"], effect["action_id"])
            if key in effects:
                require(effects[key]["values"] == effect["values"], "conflicting historical effect")
            else:
                expected_effect = effect
                effects[key] = effect
        require(same(record["new_effect"], expected_effect), "effect missing, forged or duplicated")
        records.append(record)
        receipts.append({"sequence": sequence, "receipt_sha256": receipt, "effect_count": len(effects),
                         "new_effect": expected_effect is not None, "durable": True, "readback_verified": True})
        previous = receipt
        at += 68 + length
    return records, receipts, effects, previous


def confined(root, relative):
    path = (root / relative).resolve()
    require(path.is_relative_to(root.resolve()), "path escapes capsule: " + str(relative))
    return path


def verify_manifest(path):
    seen = set()
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if not line or line.startswith("#"):
            continue
        match = re.fullmatch(r"([a-fA-F0-9]{64}) [ *](.+)", line)
        require(match is not None, "malformed SHA256SUMS line")
        expected, relative = match.groups()
        require(relative not in seen, "duplicate manifest entry")
        seen.add(relative)
        actual = hashlib.sha256(confined(path.parent, relative).read_bytes()).hexdigest()
        require(actual == expected.lower(), "manifest mismatch: " + relative)
    require(bool(seen), "empty manifest")
    return len(seen)


def verify_provenance():
    checked, unavailable = [], []
    def check(path, expected, label):
        require(hashlib.sha256(path.read_bytes()).hexdigest() == expected.lower(), "provenance hash: " + label)
        checked.append(label)

    mapping = ROOT / "provenance" / "ffi_zero_noi.json"
    if mapping.exists():
        manifest = load(mapping)
        for name in ["ffi", "zero_noi"]:
            entry = manifest[name]
            extracted = confined(ROOT, entry["extracted_path"])
            check(extracted, entry["extracted_sha256"], name + " extraction")
            if "wrapper_sha256" in entry:
                check(confined(ROOT, entry["wrapper"]), entry["wrapper_sha256"], name + " wrapper")
            source = Path(entry["source"])
            if not source.is_file():
                unavailable.append(name + " original source")
                continue
            check(source, entry["source_sha256"], name + " original source")
            raw = source.read_bytes()
            if "line_ranges_inclusive" in entry:
                lines = raw.splitlines(keepends=True)
                fragments = []
                for start, end in entry["line_ranges_inclusive"]:
                    require(1 <= start <= end <= len(lines), "invalid extraction range")
                    fragments.extend(lines[start - 1:end])
                raw = b"".join(fragments)
            require(raw == extracted.read_bytes(), name + " extraction is not exact")
            checked.append(name + " exact source bytes")

    project = ROOT.parents[1]
    source_roots = {
        "C": project / "v1.8.9_ess_mai_phase_015-ws/v1.8.9_ess_mai_phase_015/ess_mai",
        "A": project.parent / "aig-c/codex_zvn/runtime",
    }
    hardware = ROOT / "provenance" / "hardware.json"
    if hardware.exists():
        for entry in load(hardware)["sources"]:
            root = source_roots["C" if entry["system"].startswith("ESS-MAI") else "A"]
            source = root / entry["path"]
            if not source.is_file():
                unavailable.append(entry["system"] + " hardware source")
                continue
            check(source, entry["file_sha256"], entry["system"] + " hardware source")
            start, end = entry["lines"]
            lines = source.read_text(encoding="utf-8").splitlines()
            require(1 <= start <= end <= len(lines), "hardware extraction range")
            block = ("\n".join(lines[start - 1:end]) + "\n").encode("utf-8")
            require(hashlib.sha256(block).hexdigest() == entry["block_sha256_lf_utf8"], "hardware block hash")
            checked.append(entry["system"] + " hardware block")

    methodology = ROOT / "provenance" / "methodology.json"
    if methodology.exists():
        def walk(value):
            if isinstance(value, list):
                for item in value:
                    walk(item)
            elif isinstance(value, dict):
                if {"source_root", "relative_path", "whole_file_sha256"} <= set(value):
                    source = source_roots[value["source_root"]] / value["relative_path"]
                    label = value.get("id", value["relative_path"])
                    if source.is_file():
                        check(source, value["whole_file_sha256"], label)
                    else:
                        unavailable.append(label)
                else:
                    for item in value.values():
                        walk(item)
        walk(load(methodology))
    return {"checked": checked, "unavailable_original_sources": unavailable}


def verify_demo(directory):
    raw = (directory / "memory.wal").read_bytes()
    records, commits, effects, head = decode_wal(raw)
    require(len(records) == 5, "expected exactly five cycles")
    require([r["report"]["verification"] for r in records] == BITS, "unexpected five-cycle bits")
    require(len({context_hash(r["report"]["input"]) for r in records}) == 1, "continuity context changed")
    require(len(effects) == 2, "expected two unique positive effects")
    for record, commit in zip(records, commits):
        number = record["sequence"]
        require(same(validate_input(load(directory / f"input-{number}.json")), record["report"]["input"]), "input file differs from WAL")
        output = canonical(load(directory / f"cycle-{number}.json"), "CycleOutput")
        require(same(output["report"], record["report"]) and same(output["commit"], commit), "cycle/report/commit mismatch")
        require(output["gcl_restart_after_durable_commit"] is True
                and type(output["next_cycle"]) is int and output["next_cycle"] == number + 1, "commit/reset order evidence")
    return {"cycles": 5, "verification_bits": BITS, "unique_effects": len(effects), "head_sha256": head}, raw, records


def self_test(directory=None):
    count = 0
    def rejects(fn):
        nonlocal count
        try:
            fn()
        except (Invalid, ValueError, KeyError, UnicodeError, struct.error):
            count += 1
            return
        raise Invalid("self-test mutation was accepted")
    manual = b"\x01\x00\x00\x00\x00\x00\x00\x00d\x02\x00\x00\x00\x00\x00\x00\x00ab"
    require(hash_parts("d", b"ab") == hashlib.sha256(manual).hexdigest(), "length-prefix test")
    count += 1
    rejects(lambda: parse('{"x":1,"x":2}'))
    rejects(lambda: parse('{"x":NaN}'))
    rejects(lambda: decode_wal(MAGIC + b"x"))
    rejects(lambda: integer(True, 0, 1, "boolean is not verification"))
    fixture = validate_input({
        "cycle": 1, "context": "self-test", "scope": SCOPE, "action_id": "test",
        "question": "Check bounded affine models.",
        "law": {"jurisdiction": SCOPE, "risk_limit": 10, "byte_budget": 4096,
                "time_budget_ns": 1_000_000_000},
        "candidates": [{"id": name, "a": a, "b": 1,
                        "samples": [{"x": x, "y": a * x + 1} for x in DOMAIN]}
                       for name, a in [("safe", 2), ("unsafe", 4)]],
    })
    evaluations = assess(fixture)
    require(evaluations[0]["values"] == [1, 3, 5, 7, 9]
            and evaluations[0]["risk"] == 9 and evaluations[0]["gain"] == 25,
            "independent affine arithmetic self-test")
    require(evaluations[1]["risk"] == 17 and evaluations[1]["gain"] == 45
            and evaluations[1]["status"] == "EliminatedCounterproof"
            and winner(evaluations)["candidate_id"] == "safe", "hard bound cannot be offset by gain")
    count += 2
    fixture["candidates"][0]["samples"].pop()
    require(assess(fixture)[0]["status"] == "MissingEvidence"
            and winner(assess(fixture)) is None, "missing evidence is not counterproof")
    count += 1
    if directory:
        _, raw, records = verify_demo(directory)
        rejects(lambda: decode_wal(raw[:-1]))
        damaged = bytearray(raw)
        damaged[-1] ^= 1
        rejects(lambda: decode_wal(bytes(damaged)))
        for field in ["gain", "payload", "bit", "seal", "deadline", "memory", "question"]:
            changed = copy.deepcopy(records[3]["report"])
            if field == "gain":
                changed["assessments"][0]["gain"] += 1
            elif field == "payload":
                changed["hardware"]["payload_sha256"] = "0" * 64
            elif field == "bit":
                changed["verification"] = [1, 1]
            elif field == "deadline":
                changed["hardware"]["elapsed_ns"] = changed["hardware"]["time_budget_ns"] + 1
            elif field == "memory":
                changed["lgc"]["steps"][2]["payload"]["memory_refs"][0]["receipt_sha256"] = "0" * 64
            elif field == "question":
                changed["questions"] = []
            else:
                changed["seal_500"] = "0" * 64
            rejects(lambda: verify_report(changed, records[3]["previous_receipt_sha256"], records[:3]))
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", nargs="?", type=Path, help="five-cycle evidence directory")
    parser.add_argument("--manifest", type=Path, help="optional sha256sum-format manifest")
    parser.add_argument("--self-test", action="store_true", help="run parser and evidence tampering checks")
    args = parser.parse_args()
    require(args.directory is not None or args.self_test, "provide evidence directory or --self-test")
    result = {"status": "PASS", "verifier": "Python stdlib independent bounded-model verification",
              "native_paired_verdict_claim": False,
              "limitations": "Checks bytes, model mathematics and reported observations; does not authenticate a hostile host, remeasure timing, or prove FFI consumption from logs alone."}
    if args.directory:
        result["evidence"], _, _ = verify_demo(args.directory)
        result["provenance"] = verify_provenance()
    manifest = args.manifest
    if manifest is None:
        manifest = next((ROOT / name for name in ["SHA256SUMS", "SHA256SUMS.txt"] if (ROOT / name).is_file()), None)
    if manifest:
        result["manifest_files_checked"] = verify_manifest(manifest)
    if args.self_test:
        result["self_tests_passed"] = self_test(args.directory)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (Invalid, OSError, ValueError, KeyError, TypeError, UnicodeError, struct.error) as error:
        print(json.dumps({"status": "FAIL", "error": str(error)}), file=sys.stderr)
        sys.exit(1)
