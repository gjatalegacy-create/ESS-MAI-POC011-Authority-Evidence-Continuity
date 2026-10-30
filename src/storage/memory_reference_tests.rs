use super::*;
use crate::authority::MemoryReference;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

fn temporary_path() -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "poc011-memory-refs-{}-{}-{}.wal",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn five_record_fixture() -> (Vec<Record>, Vec<u8>) {
    let path = temporary_path();
    let mut engine = crate::Engine::create(&path).unwrap();
    for input in crate::demonstration_inputs() {
        engine.process(&input).unwrap();
    }
    drop(engine);
    let original = std::fs::read(&path).unwrap();
    assert!(Store::open(&path).is_ok());
    std::fs::remove_file(path).unwrap();
    (decode(&original).unwrap().0, original)
}

// Match the typed coordination encoding solely to reconstruct all dependent
// checksums. The adversarial record must remain structurally valid so a failing
// test proves the missing history comparison, not a broken checksum.
#[derive(Serialize, Deserialize)]
struct CoordinationEncoding {
    input_sha256: String,
    context_sha256: String,
    previous_cycle_sha256: String,
    candidate_hashes: Vec<String>,
    memory_refs: Vec<MemoryReference>,
}

fn rebuild_trace(report: &mut Report) {
    let old = report.lgc.clone();
    let coordination: CoordinationEncoding =
        serde_json::from_value(old.steps[2].payload.clone()).unwrap();
    let mut trace = crate::trace::Lgc::new(&report.input, &old.previous_cycle_sha256);
    for (index, mut step) in old.steps.into_iter().enumerate() {
        if index == 3 {
            step.payload["evidence"]["coordination_sha256"] =
                json!(hash_value("POC011_COORDINATION_V1", &coordination));
        }
        if index == 7 {
            step.payload["evidence"]["received_tip"] = json!(trace.tip());
        }
        trace.append(&step.role, &step.operation, step.payload);
    }
    report.lgc = trace;
    report.seal_500 = Some(hash_value(
        "POC011_SEAL_500_V1",
        &(500, report.lgc.tip(), report.input.input_sha256()),
    ));
    assert!(report.lgc.verify());
    assert!(crate::authority::replay_trace_valid(report));
    assert!(validate_report(report).is_ok());
}

fn frame_records(records: &[Record]) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    let mut previous = genesis();
    for record in records {
        assert_eq!(record.previous_receipt_sha256, previous);
        let body = serde_json::to_vec(record).unwrap();
        let length = (body.len() as u32).to_le_bytes();
        let receipt = hash_parts(
            "POC011_WAL_FRAME_V1",
            &[previous.as_bytes(), &length, &body],
        );
        bytes.extend_from_slice(&length);
        bytes.extend_from_slice(receipt.as_bytes());
        bytes.extend_from_slice(&body);
        previous = receipt;
    }
    bytes
}

fn rejects_rehashed_last_reference(change: impl FnOnce(&mut Value)) {
    let (mut records, original) = five_record_fixture();
    let prefix = frame_records(&records[..4]);
    assert_eq!(prefix, original[..prefix.len()]);
    let report = &mut records.last_mut().unwrap().report;
    change(&mut report.lgc.steps[2].payload["memory_refs"]);
    rebuild_trace(report);
    let changed = frame_records(&records);
    assert_eq!(prefix, changed[..prefix.len()]);
    let path = temporary_path();
    std::fs::write(&path, &changed).unwrap();
    let decode_rejected = decode(&changed).is_err();
    let reopen_rejected = Store::open(&path).is_err();
    assert_eq!(std::fs::read(&path).unwrap(), changed);
    std::fs::remove_file(path).unwrap();
    assert!(
        decode_rejected,
        "rehashed reference contradicts unchanged history"
    );
    assert!(
        reopen_rejected,
        "Store::open admitted inconsistent prior references"
    );
}

#[test]
fn rehashed_wrong_prior_receipt_refuses_reopen() {
    rejects_rehashed_last_reference(|refs| refs[0]["receipt_sha256"] = json!("0".repeat(64)));
}

#[test]
fn rehashed_dropped_history_references_refuse_reopen() {
    rejects_rehashed_last_reference(|refs| *refs = json!([]));
}

#[test]
fn rehashed_invented_negative_reference_refuses_reopen() {
    rejects_rehashed_last_reference(|refs| {
        assert_eq!(refs[0]["negative_candidate_sha256"], json!([]));
        refs[0]["negative_candidate_sha256"] = json!(["a".repeat(64)]);
    });
}

#[test]
fn rehashed_changed_positive_reference_refuses_reopen() {
    rejects_rehashed_last_reference(|refs| {
        assert_eq!(
            refs[3]["positive_candidate_sha256"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        refs[3]["positive_candidate_sha256"][0] = json!("b".repeat(64));
    });
}

#[test]
fn rehashed_wrong_question_count_refuses_reopen() {
    rejects_rehashed_last_reference(|refs| {
        refs[0]["question_count"] = json!(refs[0]["question_count"].as_u64().unwrap() + 1);
    });
}

#[test]
fn interleaved_contexts_reopen_with_exact_last_eight_references() {
    let path = temporary_path();
    drop(crate::Engine::create(&path).unwrap());
    let mut previous: Vec<(u64, String, String)> = Vec::new();
    for cycle in 1..=20 {
        let mut input = crate::demonstration_inputs().remove(3);
        input.cycle = cycle;
        input.context = if cycle % 3 == 0 { "beta" } else { "alpha" }.into();
        input.action_id = format!("mixed-context-{cycle}");
        let mut engine = crate::Engine::open(&path).unwrap();
        let output = engine.process(&input).unwrap();
        assert_eq!(output.report.verification, 1);
        let refs: Vec<MemoryReference> =
            serde_json::from_value(output.report.lgc.steps[2].payload["memory_refs"].clone())
                .unwrap();
        let expected: Vec<_> = previous
            .iter()
            .filter(|(_, context, _)| context == &input.context)
            .collect();
        let expected = &expected[expected.len().saturating_sub(8)..];
        assert_eq!(refs.len(), expected.len());
        for (reference, (sequence, _, receipt)) in refs.iter().zip(expected) {
            assert_eq!(reference.cycle, *sequence);
            assert_eq!(&reference.receipt_sha256, receipt);
            assert_eq!(reference.context_sha256, input.context_sha256());
        }
        if cycle == 20 {
            assert_eq!(
                refs.iter()
                    .map(|reference| reference.cycle)
                    .collect::<Vec<_>>(),
                [8, 10, 11, 13, 14, 16, 17, 19]
            );
        }
        previous.push((cycle, input.context, output.commit.receipt_sha256));
    }
    let mut input = crate::demonstration_inputs().remove(3);
    input.cycle = 21;
    input.context = "alpha".into();
    input.action_id = "current-evidence-still-required".into();
    for candidate in &mut input.candidates {
        candidate.samples.clear();
    }
    let mut engine = crate::Engine::open(&path).unwrap();
    let output = engine.process(&input).unwrap();
    assert_eq!(output.report.verification, 0);
    assert_eq!(output.report.status, "HoldMissingEvidence");
    assert_eq!(output.commit.effect_count, 20);
    assert!(!output.commit.new_effect);
    drop(engine);
    assert_eq!(Store::open(&path).unwrap().records.len(), 21);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn genuine_integrity_incident_without_coordination_can_commit_and_reopen() {
    let path = temporary_path();
    let mut store = Store::create(&path).unwrap();
    let mut input = crate::demonstration_inputs().remove(3);
    input.cycle = 1;
    let invalid_reference = MemoryReference {
        cycle: 99,
        context_sha256: input.context_sha256(),
        receipt_sha256: "0".repeat(64),
        negative_candidate_sha256: vec![],
        positive_candidate_sha256: vec![],
        question_count: 0,
    };
    let closure = crate::authority::run_with_memory(
        &input,
        store.head(),
        &mut crate::ffi::ActionGate::new(),
        vec![invalid_reference],
    )
    .unwrap();
    let (report, commit) = store.commit(closure).unwrap();
    assert_eq!(report.status, "FailClosedIntegrity");
    assert_eq!(report.verification, 0);
    assert_eq!(report.lgc.steps.len(), 5);
    assert_eq!(report.lgc.steps[0].operation, "received_invalid_trace");
    assert!(report.seal_500.is_none());
    assert!(!commit.new_effect);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.records.len(), 1);
    assert_eq!(reopened.effect_count(), 0);
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn commit_rejects_false_history_before_any_write_or_state_change() {
    let path = temporary_path();
    let mut store = Store::create(&path).unwrap();
    let mut input = crate::demonstration_inputs().remove(3);
    input.cycle = 1;
    let closure = crate::authority::run_with_memory(
        &input,
        store.head(),
        &mut crate::ffi::ActionGate::new(),
        vec![],
    )
    .unwrap();
    let (first, _) = store.commit(closure).unwrap();
    assert_eq!(first.verification, 1);

    let false_reference = MemoryReference {
        cycle: 1,
        context_sha256: input.context_sha256(),
        receipt_sha256: "0".repeat(64),
        negative_candidate_sha256: first
            .assessments
            .iter()
            .filter(|a| a.status == "EliminatedCounterproof")
            .map(|a| a.candidate_sha256.clone())
            .collect(),
        positive_candidate_sha256: first.selected_candidate_sha256.iter().cloned().collect(),
        question_count: first.questions.len(),
    };
    assert_ne!(false_reference.receipt_sha256, store.head());
    input.cycle = 2;
    input.action_id = "second-current-valid-action".into();

    // Current proof remains SHADOW's task. Historical receipt consistency is
    // independently required by storage before it accepts a new durable record.
    let current_report = crate::authority::run_with_memory(
        &input,
        store.head(),
        &mut crate::ffi::ActionGate::new(),
        vec![false_reference.clone()],
    )
    .unwrap()
    .into_report();
    assert_eq!(current_report.status, "VerifiedAction");
    assert_eq!(current_report.verification, 1);
    let closure = crate::authority::run_with_memory(
        &input,
        store.head(),
        &mut crate::ffi::ActionGate::new(),
        vec![false_reference],
    )
    .unwrap();

    let before_head = store.head().to_string();
    let before_records = store.records.clone();
    let before_effects = store.effects.clone();
    store.file.seek(SeekFrom::Start(0)).unwrap();
    let mut before_bytes = Vec::new();
    store.file.read_to_end(&mut before_bytes).unwrap();
    let refused = store.commit(closure).is_err();
    store.file.seek(SeekFrom::Start(0)).unwrap();
    let mut after_bytes = Vec::new();
    store.file.read_to_end(&mut after_bytes).unwrap();
    let unchanged = before_bytes == after_bytes
        && store.head() == before_head
        && store.records == before_records
        && store.effects == before_effects;
    drop(store);
    std::fs::remove_file(path).unwrap();
    assert!(refused, "commit admitted a false historical receipt");
    assert!(unchanged, "refused commit changed WAL or in-memory state");
}
