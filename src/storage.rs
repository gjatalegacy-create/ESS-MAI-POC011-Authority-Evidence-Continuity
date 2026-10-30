use crate::authority::{Report, VerifiedClosure};
use crate::{hash_parts, hash_value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const MAGIC: &[u8; 8] = b"POC011\0\x01";
const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_FRAME: usize = 256 * 1024;
pub(crate) fn genesis() -> String {
    hash_parts("POC011_WAL_GENESIS_V1", &[MAGIC])
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub action_id: String,
    pub context_sha256: String,
    pub candidate_sha256: String,
    pub values: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub version: u8,
    pub sequence: u64,
    pub previous_receipt_sha256: String,
    pub report: Report,
    pub new_effect: Option<Effect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    pub sequence: u64,
    pub receipt_sha256: String,
    pub effect_count: u64,
    pub new_effect: bool,
    pub durable: bool,
    pub readback_verified: bool,
}

pub(crate) struct Store {
    file: File,
    pub(crate) records: Vec<Record>,
    head: String,
    effects: BTreeMap<(String, String), Effect>,
}

fn io(e: std::io::Error) -> String {
    format!("storage: {e}")
}

impl Store {
    pub(crate) fn create(path: &Path) -> Result<Self, String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(io)?;
        file.try_lock()
            .map_err(|e| format!("storage locked: {e}"))?;
        file.write_all(MAGIC).map_err(io)?;
        file.sync_all().map_err(io)?;
        Ok(Self {
            file,
            records: vec![],
            head: genesis(),
            effects: BTreeMap::new(),
        })
    }
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(io)?;
        file.try_lock()
            .map_err(|e| format!("storage locked: {e}"))?;
        if file.metadata().map_err(io)?.len() > MAX_FILE {
            return Err("WAL limit exceeded".into());
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(io)?;
        let (records, head, effects) = decode(&bytes)?;
        // Reconcile a complete frame left before an interrupted writer's acknowledgment.
        file.sync_all().map_err(io)?;
        Ok(Self {
            file,
            records,
            head,
            effects,
        })
    }
    pub(crate) fn head(&self) -> &str {
        &self.head
    }
    pub(crate) fn effect_count(&self) -> u64 {
        self.effects.len() as u64
    }
    pub(crate) fn next_cycle(&self) -> u64 {
        self.records.len() as u64 + 1
    }
    pub(crate) fn commit(&mut self, closure: VerifiedClosure) -> Result<(Report, Commit), String> {
        let report = closure.into_report();
        if report.input.cycle != self.next_cycle() || report.lgc.previous_cycle_sha256 != self.head
        {
            return Err("stale cycle or predecessor".into());
        }
        let prospective = effect_for(&report)?;
        let new_effect = match prospective {
            Some(e) => match self
                .effects
                .get(&(e.context_sha256.clone(), e.action_id.clone()))
            {
                Some(prior) if prior.values == e.values => None,
                Some(_) => {
                    return Err("conflicting replay: action already has different effect".into())
                }
                None => Some(e),
            },
            None => None,
        };
        let record = Record {
            version: 1,
            sequence: report.input.cycle,
            previous_receipt_sha256: self.head.clone(),
            report: report.clone(),
            new_effect,
        };
        let body = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
        if body.len() > MAX_FRAME || self.records.len() >= 1024 {
            return Err("bounded journal exhausted".into());
        }
        let len = (body.len() as u32).to_le_bytes();
        let receipt = hash_parts("POC011_WAL_FRAME_V1", &[self.head.as_bytes(), &len, &body]);
        let mut frame = Vec::new();
        frame.extend_from_slice(&len);
        frame.extend_from_slice(receipt.as_bytes());
        frame.extend_from_slice(&body);
        let start = self.file.seek(SeekFrom::End(0)).map_err(io)?;
        if start + frame.len() as u64 > MAX_FILE {
            return Err("WAL limit exceeded".into());
        }
        self.file.write_all(&frame).map_err(io)?;
        self.file.flush().map_err(io)?;
        self.file.sync_all().map_err(io)?;
        self.file.seek(SeekFrom::Start(start)).map_err(io)?;
        let mut readback = vec![0; frame.len()];
        self.file.read_exact(&mut readback).map_err(io)?;
        if readback != frame {
            return Err("durable frame readback mismatch".into());
        }
        if let Some(e) = &record.new_effect {
            self.effects
                .insert((e.context_sha256.clone(), e.action_id.clone()), e.clone());
        }
        self.head = receipt.clone();
        self.records.push(record.clone());
        Ok((
            report,
            Commit {
                sequence: record.sequence,
                receipt_sha256: receipt,
                effect_count: self.effect_count(),
                new_effect: record.new_effect.is_some(),
                durable: true,
                readback_verified: true,
            },
        ))
    }
}

const GATE_NAMES: [&str; 9] = [
    "Spine.scope",
    "Spine.law_complete",
    "Spine.lgc_exact",
    "Spine.current_evidence",
    "Spine.authority_order",
    "Spine.hardware_readback",
    "Spine.quantum_completion",
    "Matrix.current_dossier",
    "Judiciary.single_use_local_ffi",
];

// Replay checks recorded consistency, not host authenticity or a new hardware execution.
// A fully rewritten, internally valid history still needs an external trusted anchor.
fn validate_report(r: &Report) -> Result<(), String> {
    use crate::domain::{assess, select};
    r.input.validate_shape()?;
    if r.verification > 1
        || !r.lgc.verify()
        || !crate::authority::replay_trace_valid(r)
        || r.lgc.input_sha256 != r.input.input_sha256()
        || r.lgc.context_sha256 != r.input.context_sha256()
        || r.lgc.steps.is_empty()
        || r.lgc.steps.len() > 32
        || r.gates.iter().map(|g| g.name.as_str()).collect::<Vec<_>>() != GATE_NAMES
        || r.output.is_empty()
        || r.output.len() > 8192
        || r.questions.len() > 64
        || r.questions.iter().any(|q| q.is_empty() || q.len() > 8192)
    {
        return Err("report identity, trace or fields invalid".into());
    }
    if r.status == "FailClosedIntegrity" {
        if r.verification != 0
            || r.seal_500.is_some()
            || r.gates.iter().all(|g| g.passed)
            || r.questions.is_empty()
            || r.lgc.steps[0].role != "SHADOW"
            || r.lgc.steps[0].operation != "received_invalid_trace"
            || r.lgc.steps[0].payload.get("untrusted_trace").is_none()
        {
            return Err("invalid integrity incident".into());
        }
        return Ok(());
    }
    if r.seal_500
        != Some(hash_value(
            "POC011_SEAL_500_V1",
            &(500, r.lgc.tip(), r.input.input_sha256()),
        ))
    {
        return Err("report seal mismatch".into());
    }
    let gaps = r.input.law.gaps();
    let assessments = if gaps.is_empty() {
        assess(&r.input)
    } else {
        Vec::new()
    };
    let selected = select(&assessments);
    if r.assessments != assessments
        || r.selected_candidate_sha256.as_deref() != selected.map(|a| a.candidate_sha256.as_str())
    {
        return Err("recorded reasoning disagrees with bounded input".into());
    }
    let receipt: Option<crate::hardware::HardwareReceipt> =
        serde_json::from_value(r.hardware.clone())
            .map_err(|_| "invalid hardware receipt encoding")?;
    if serde_json::to_value(&receipt).map_err(|e| e.to_string())? != r.hardware {
        return Err("noncanonical hardware receipt".into());
    }
    let hardware_ok = match (selected, receipt.as_ref()) {
        (Some(a), Some(h)) => {
            use sha2::{Digest, Sha256};
            let bytes: Vec<u8> = a.values.iter().flat_map(|v| v.to_le_bytes()).collect();
            h.verified_readback
                && h.payload_sha256 == format!("{:x}", Sha256::digest(&bytes))
                && h.requested_bytes == bytes.len() as u64
                && h.actual_bytes == bytes.len() as u64
                && Some(h.byte_budget) == r.input.law.byte_budget
                && h.actual_bytes <= h.byte_budget
                && Some(h.time_budget_ns) == r.input.law.time_budget_ns
                && h.time_budget_ns > 0
                && h.elapsed_ns <= h.time_budget_ns
                && h.physical_energy_joules.is_none()
        }
        _ => false,
    };
    if receipt.is_some() && !hardware_ok {
        return Err("invalid recorded hardware evidence".into());
    }
    let expected_status = if !gaps.is_empty() {
        "HoldIncompleteLaw"
    } else if selected.is_some() {
        if hardware_ok {
            "VerifiedAction"
        } else {
            "HoldHardwareEvidence"
        }
    } else if assessments.iter().any(|a| a.status == "MissingEvidence") {
        "HoldMissingEvidence"
    } else if assessments.is_empty() {
        "FailClosedNoCandidates"
    } else {
        "FailClosedCounterproof"
    };
    let verified = expected_status == "VerifiedAction";
    let expected_gates = [
        true,
        gaps.is_empty(),
        true,
        true,
        true,
        hardware_ok,
        verified,
        selected.is_some(),
        verified,
    ];
    if r.status != expected_status
        || r.verification != u8::from(verified)
        || r.gates.iter().map(|g| g.passed).collect::<Vec<_>>() != expected_gates
    {
        return Err("report status or required gates inconsistent".into());
    }
    let reasoning: Vec<_> = r
        .lgc
        .steps
        .iter()
        .filter(|s| s.role == "QUANTUM" && s.operation == "eliminate")
        .collect();
    if reasoning.len() != 1 {
        return Err("missing or duplicate recorded reasoning".into());
    }
    let q = &reasoning[0].payload;
    let quantum_status = if verified {
        "CandidateReady"
    } else {
        expected_status
    };
    if q.get("assessments") != Some(&serde_json::json!(r.assessments))
        || q.get("selected") != Some(&serde_json::json!(r.selected_candidate_sha256))
        || q.get("hardware") != Some(&r.hardware)
        || q.get("status").and_then(serde_json::Value::as_str) != Some(quantum_status)
    {
        return Err("trace/report reasoning mismatch".into());
    }
    let hardware_failure = q.get("hardware_failure").ok_or("hardware outcome absent")?;
    let failure = if expected_status == "HoldHardwareEvidence" {
        Some(
            hardware_failure
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 8192)
                .ok_or("hardware refusal cause absent")?,
        )
    } else {
        if !hardware_failure.is_null() {
            return Err("unexpected hardware refusal".into());
        }
        None
    };
    let mut questions: Vec<String> = gaps
        .iter()
        .map(|g| {
            format!(
                "What is the applicable value for {g} in scope {}?",
                r.input.scope
            )
        })
        .collect();
    for a in &assessments {
        if !a.missing_x.is_empty() {
            questions.push(format!(
                "Provide current y evidence for candidate {} at x={:?} under y=a*x+b.",
                a.candidate_id, a.missing_x
            ));
        }
        for ce in &a.counterexamples {
            questions.push(format!(
            "{ce}. What corrected candidate or evidence addresses this exact counterexample within the same law?"));
        }
    }
    if assessments.is_empty() && gaps.is_empty() {
        questions.push(
            "Which candidate and complete model table should LIGHT coordinate for this context?"
                .into(),
        );
    }
    if let Some(e) = failure {
        questions.push(format!(
        "Hardware execution lacks a valid receipt ({e}); what sufficient measured execution budget can be provided?"));
    }
    let output = if verified {
        format!(
            "Verified bounded action: {}. Scope: {}. No claim beyond the finite model.",
            r.selected_candidate_sha256.as_deref().unwrap_or(""),
            r.input.scope
        )
    } else {
        format!(
            "{expected_status}: no action admitted. Current evidence and its gaps remain explicit."
        )
    };
    if r.questions != questions || r.output != output {
        return Err("output/question derivation mismatch".into());
    }
    Ok(())
}

fn effect_for(r: &Report) -> Result<Option<Effect>, String> {
    validate_report(r)?;
    if r.verification == 0 {
        return Ok(None);
    }
    let a = crate::domain::select(&r.assessments).ok_or("selected candidate missing")?;
    Ok(Some(Effect {
        action_id: r.input.action_id.clone(),
        context_sha256: r.input.context_sha256(),
        candidate_sha256: a.candidate_sha256.clone(),
        values: a.values.clone(),
    }))
}

type Decoded = (Vec<Record>, String, BTreeMap<(String, String), Effect>);
fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    if bytes.get(..8) != Some(MAGIC.as_slice()) {
        return Err("invalid journal header".into());
    }
    let mut records = Vec::new();
    let mut head = genesis();
    let mut effects = BTreeMap::<(String, String), Effect>::new();
    let mut at = 8;
    while at < bytes.len() {
        if records.len() >= 1024 || bytes.len() - at < 68 {
            return Err("incomplete journal frame".into());
        }
        let len = u32::from_le_bytes(bytes[at..at + 4].try_into().map_err(|_| "length")?) as usize;
        if len == 0 || len > MAX_FRAME || bytes.len() - at - 68 < len {
            return Err("invalid/truncated frame".into());
        }
        let receipt =
            std::str::from_utf8(&bytes[at + 4..at + 68]).map_err(|_| "invalid hash bytes")?;
        let body = &bytes[at + 68..at + 68 + len];
        if receipt
            != hash_parts(
                "POC011_WAL_FRAME_V1",
                &[head.as_bytes(), &bytes[at..at + 4], body],
            )
        {
            return Err("WAL SHA-256 mismatch".into());
        }
        let r: Record = serde_json::from_slice(body).map_err(|e| format!("invalid record: {e}"))?;
        if serde_json::to_vec(&r).map_err(|e| e.to_string())? != body {
            return Err("noncanonical record encoding".into());
        }
        r.report.input.validate_shape()?;
        if r.version != 1
            || r.sequence != records.len() as u64 + 1
            || r.report.input.cycle != r.sequence
            || r.previous_receipt_sha256 != head
            || r.report.lgc.previous_cycle_sha256 != head
            || r.report.lgc.input_sha256 != r.report.input.input_sha256()
            || r.report.lgc.context_sha256 != r.report.input.context_sha256()
        {
            return Err("record identity/sequence mismatch".into());
        }
        let proposed = effect_for(&r.report)?;
        let expected = if let Some(e) = proposed {
            match effects.get(&(e.context_sha256.clone(), e.action_id.clone())) {
                Some(prior) if prior.values == e.values => None,
                Some(_) => return Err("conflicting historical effect".into()),
                None => Some(e),
            }
        } else {
            None
        };
        if expected != r.new_effect {
            return Err("effect/report inconsistency".into());
        }
        if let Some(e) = &r.new_effect {
            effects.insert((e.context_sha256.clone(), e.action_id.clone()), e.clone());
        }
        head = receipt.to_string();
        records.push(r);
        at += 68 + len;
    }
    Ok((records, head, effects))
}

pub fn inspect(path: &Path) -> Result<serde_json::Value, String> {
    let mut file = File::open(path).map_err(io)?;
    file.try_lock_shared()
        .map_err(|e| format!("storage locked: {e}"))?;
    if file.metadata().map_err(io)?.len() > MAX_FILE {
        return Err("WAL limit exceeded".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(io)?;
    let (records, head, effects) = decode(&bytes)?;
    Ok(
        serde_json::json!({"records":records,"head_sha256":head,"effect_count":effects.len(),"interpretation":"storage integrity replay; not a new SHADOW decision"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record_fixture(index: usize) -> Record {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "poc011-replay-{}-{}-{}.wal",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut engine = crate::Engine::create(&path).unwrap();
        let mut input = crate::demonstration_inputs().remove(index);
        input.cycle = 1;
        engine.process(&input).unwrap();
        drop(engine);
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        decode(&bytes).unwrap().0.remove(0)
    }

    fn reframe(record: &Record) -> Vec<u8> {
        let body = serde_json::to_vec(record).unwrap();
        let len = (body.len() as u32).to_le_bytes();
        let receipt = hash_parts("POC011_WAL_FRAME_V1", &[genesis().as_bytes(), &len, &body]);
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&len);
        bytes.extend_from_slice(receipt.as_bytes());
        bytes.extend_from_slice(&body);
        bytes
    }

    #[test]
    fn rehashed_refusal_with_broken_lgc_is_rejected() {
        let mut record = record_fixture(0);
        assert_eq!(record.report.verification, 0);
        record.report.lgc.steps[0].payload = serde_json::json!({"changed":true});
        assert!(decode(&reframe(&record)).is_err());
    }

    #[test]
    fn rehashed_positive_with_changed_assessment_is_rejected() {
        let mut record = record_fixture(3);
        assert_eq!(record.report.verification, 1);
        record.report.assessments[0].gain = Some(12345);
        assert!(decode(&reframe(&record)).is_err());
    }

    #[test]
    fn rebuilt_hash_chain_cannot_transfer_a_witness_role() {
        for index in [0, 3] {
            let mut record = record_fixture(index);
            let old = record.report.lgc.clone();
            let mut changed =
                crate::trace::Lgc::new(&record.report.input, &old.previous_cycle_sha256);
            for (n, step) in old.steps.into_iter().enumerate() {
                changed.append(
                    if n == 0 { "LIGHT" } else { &step.role },
                    &step.operation,
                    step.payload,
                );
            }
            record.report.lgc = changed;
            record.report.seal_500 = Some(hash_value(
                "POC011_SEAL_500_V1",
                &(
                    500,
                    record.report.lgc.tip(),
                    record.report.input.input_sha256(),
                ),
            ));
            assert!(decode(&reframe(&record)).is_err());
        }
    }

    #[test]
    fn trailing_and_truncated_bytes_are_rejected() {
        assert!(decode(MAGIC).is_ok());
        for tail in [vec![0], vec![0; 67], vec![255; 68]] {
            let mut b = MAGIC.to_vec();
            b.extend(tail);
            assert!(decode(&b).is_err());
        }
    }
}
