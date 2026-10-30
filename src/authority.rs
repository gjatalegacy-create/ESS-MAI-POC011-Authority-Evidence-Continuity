use crate::domain::{assess, select, Assessment};
use crate::ffi::ActionGate;
use crate::hardware::{self, HardwareReceipt};
use crate::trace::Lgc;
use crate::{hash_value, Input};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub name: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub input: Input,
    pub status: String,
    pub verification: u8,
    pub selected_candidate_sha256: Option<String>,
    pub assessments: Vec<Assessment>,
    pub hardware: Value,
    pub gates: Vec<Gate>,
    pub seal_500: Option<String>,
    pub lgc: Lgc,
    pub output: String,
    pub questions: Vec<String>,
}

/// A closure authorizes only a journal write. A scalar 0 records refusal, never an effect.
/// No public constructor, Clone or Copy; only SHADOW can produce it.
pub(crate) struct VerifiedClosure {
    report: Report,
}

/// Structural replay check only. It does not issue a new SHADOW verdict or capability.
pub(crate) fn replay_trace_valid(report: &Report) -> bool {
    let steps = &report.lgc.steps;
    let offset = if report.status == "FailClosedIntegrity" {
        if steps.len() != 5
            || steps[0].role != "SHADOW"
            || steps[0].operation != "received_invalid_trace"
        {
            return false;
        }
        1
    } else {
        if steps.len() != 13 {
            return false;
        }
        let Ok(c) = serde_json::from_value::<Coordination>(steps[2].payload.clone()) else {
            return false;
        };
        let Ok(r) = serde_json::from_value::<Reasoning>(steps[5].payload.clone()) else {
            return false;
        };
        if !coordination_valid(&report.input, &report.lgc.previous_cycle_sha256, &c) {
            return false;
        }
        let expected = trace_prefix(&report.input, &report.lgc.previous_cycle_sha256, &c, &r);
        if steps[..9] != expected.steps {
            return false;
        }
        9
    };
    if report.gates.len() != 9 {
        return false;
    }
    let spine = report.gates[..7].iter().all(|g| g.passed);
    let expected = [
        (
            "SHADOW",
            "spine",
            json!({"gates":report.gates[..7],"conjunction":spine}),
        ),
        (
            "SHADOW",
            "matrix",
            json!({"selected":report.selected_candidate_sha256,"passed":report.gates[7].passed,
            "negative_evidence_retained":report.assessments.iter().filter(|a|!a.counterexamples.is_empty()).count()}),
        ),
        (
            "SHADOW",
            "judiciary",
            json!({"verification":report.verification,"all_required":true,"ffi_consumed":report.gates[8].passed}),
        ),
        (
            "D-BAB",
            "outcome",
            json!({"owner":"SHADOW","evidence":{"status":report.status,"verification":report.verification}}),
        ),
    ];
    steps[offset..]
        .iter()
        .zip(expected)
        .all(|(s, (role, operation, payload))| {
            s.role == role && s.operation == operation && s.payload == payload
        })
}
impl VerifiedClosure {
    pub(crate) fn into_report(self) -> Report {
        self.report
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Coordination {
    input_sha256: String,
    context_sha256: String,
    previous_cycle_sha256: String,
    candidate_hashes: Vec<String>,
    memory_refs: Vec<MemoryReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct MemoryReference {
    pub cycle: u64,
    pub context_sha256: String,
    pub receipt_sha256: String,
    pub negative_candidate_sha256: Vec<String>,
    pub positive_candidate_sha256: Vec<String>,
    pub question_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Reasoning {
    assessments: Vec<Assessment>,
    status: String,
    selected: Option<String>,
    hardware: Option<HardwareReceipt>,
    hardware_failure: Option<String>,
}

fn witness(lgc: &mut Lgc, owner: &str, stage: &str, evidence: Value) {
    // D-BAB only observes the owner's conditions/result. It cannot grant or veto.
    lgc.append("D-BAB", stage, json!({"owner":owner,"evidence":evidence}));
}

fn pulse<'a>(lgc: &mut Lgc, owner: &str, input: &'a Input) -> crate::zero_noi::Pulse<'a, Input> {
    let p = crate::zero_noi::trigger(input);
    let v = p.vector();
    lgc.append(
        "0-NOI",
        "trigger",
        json!({"owner":owner,"input_sha256":p.request().input_sha256(),
        "vector":[v.state,v.knowledge_creation,v.verdict,v.authority,v.persistence,v.trigger]}),
    );
    p
}

fn coordinate(input: &Input, previous: &str) -> Coordination {
    Coordination {
        input_sha256: input.input_sha256(),
        context_sha256: input.context_sha256(),
        previous_cycle_sha256: previous.into(),
        candidate_hashes: input
            .candidates
            .iter()
            .map(|c| hash_value("POC011_CANDIDATE_V1", c))
            .collect(),
        memory_refs: Vec::new(),
    }
}

fn coordination_valid(input: &Input, previous: &str, c: &Coordination) -> bool {
    let mut expected = coordinate(input, previous);
    expected.memory_refs = c.memory_refs.clone();
    c == &expected
        && c.memory_refs.len() <= 8
        && c.memory_refs.iter().all(|r| {
            r.cycle < input.cycle
                && r.context_sha256 == input.context_sha256()
                && r.receipt_sha256.len() == 64
        })
        && c.memory_refs.windows(2).all(|r| r[0].cycle < r[1].cycle)
}

fn logical_reasoning(input: &Input) -> Reasoning {
    let mut r = Reasoning {
        assessments: vec![],
        status: "HoldIncompleteLaw".into(),
        selected: None,
        hardware: None,
        hardware_failure: None,
    };
    if !input.law.gaps().is_empty() {
        return r;
    }
    r.assessments = assess(input);
    r.selected = select(&r.assessments).map(|a| a.candidate_sha256.clone());
    r.status = if r.selected.is_some() {
        "CandidateReady"
    } else if r.assessments.iter().any(|a| a.status == "MissingEvidence") {
        "HoldMissingEvidence"
    } else if r.assessments.is_empty() {
        "FailClosedNoCandidates"
    } else {
        "FailClosedCounterproof"
    }
    .into();
    r
}

fn reason(input: &Input) -> Reasoning {
    let mut r = logical_reasoning(input);
    if let Some(selected) = select(&r.assessments) {
        match hardware::execute(
            &selected.values,
            input.law.byte_budget.unwrap_or(0),
            input.law.time_budget_ns.unwrap_or(0),
        ) {
            Ok(receipt) => r.hardware = Some(receipt),
            Err(e) => {
                r.hardware_failure = Some(e);
                r.status = "HoldHardwareEvidence".into();
            }
        }
    }
    r
}

fn trace_prefix(input: &Input, previous: &str, c: &Coordination, r: &Reasoning) -> Lgc {
    let mut lgc = Lgc::new(input, previous);
    witness(
        &mut lgc,
        "LIGHT",
        "preconditions",
        json!({"input_sha256":input.input_sha256(),"law_gaps":input.law.gaps()}),
    );
    pulse(&mut lgc, "LIGHT", input);
    lgc.append("LIGHT", "coordinate", json!(c));
    witness(
        &mut lgc,
        "QUANTUM",
        "preconditions",
        json!({"coordination_sha256":hash_value("POC011_COORDINATION_V1",c),"law_gaps":input.law.gaps()}),
    );
    pulse(&mut lgc, "QUANTUM", input);
    lgc.append("QUANTUM", "eliminate", json!(r));
    witness(
        &mut lgc,
        "QUANTUM",
        "outcome",
        json!({"status":r.status,"assessment_sha256":hash_value("POC011_ASSESSMENTS_V1",&r.assessments),"hardware_failure":r.hardware_failure}),
    );
    let received_tip = lgc.tip();
    witness(
        &mut lgc,
        "SHADOW",
        "preconditions",
        json!({"received_tip":received_tip}),
    );
    pulse(&mut lgc, "SHADOW", input);
    lgc
}

fn hardware_valid(input: &Input, r: &Reasoning) -> bool {
    let Some(candidate) = select(&r.assessments) else {
        return false;
    };
    let Some(h) = &r.hardware else {
        return false;
    };
    let bytes: Vec<u8> = candidate
        .values
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    h.verified_readback
        && h.payload_sha256 == format!("{:x}", Sha256::digest(&bytes))
        && h.requested_bytes == bytes.len() as u64
        && h.actual_bytes == bytes.len() as u64
        && Some(h.byte_budget) == input.law.byte_budget
        && h.actual_bytes <= h.byte_budget
        && Some(h.time_budget_ns) == input.law.time_budget_ns
        && h.time_budget_ns > 0
        && h.elapsed_ns <= h.time_budget_ns
        && h.physical_energy_joules.is_none()
        && r.hardware_failure.is_none()
}

fn gate(gates: &mut Vec<Gate>, name: &str, passed: bool) {
    gates.push(Gate {
        name: name.into(),
        passed,
    });
}

fn shadow(
    input: &Input,
    previous: &str,
    c: &Coordination,
    r: &Reasoning,
    mut lgc: Lgc,
    ffi: &mut ActionGate,
) -> Result<VerifiedClosure, String> {
    // Spine before Matrix, then final conjunction. No history handle enters this verifier.
    let expected = logical_reasoning(input);
    let expected_trace = trace_prefix(input, previous, c, r);
    let mut gates = Vec::new();
    gate(
        &mut gates,
        "Spine.scope",
        coordination_valid(input, previous, c),
    );
    gate(
        &mut gates,
        "Spine.law_complete",
        input.law.gaps().is_empty(),
    );
    gate(
        &mut gates,
        "Spine.lgc_exact",
        lgc.verify() && lgc == expected_trace,
    );
    gate(
        &mut gates,
        "Spine.current_evidence",
        r.assessments == expected.assessments && r.selected == expected.selected,
    );
    gate(
        &mut gates,
        "Spine.authority_order",
        lgc.steps
            .iter()
            .map(|s| s.role.as_str())
            .collect::<Vec<_>>()
            == vec![
                "D-BAB", "0-NOI", "LIGHT", "D-BAB", "0-NOI", "QUANTUM", "D-BAB", "D-BAB", "0-NOI",
            ],
    );
    gate(
        &mut gates,
        "Spine.hardware_readback",
        hardware_valid(input, r),
    );
    gate(
        &mut gates,
        "Spine.quantum_completion",
        r.status == "CandidateReady" && expected.status == "CandidateReady",
    );
    let outcome_consistent = if expected.status == "CandidateReady" {
        (r.status == "CandidateReady" && r.hardware.is_some() && r.hardware_failure.is_none())
            || (r.status == "HoldHardwareEvidence"
                && r.hardware.is_none()
                && r.hardware_failure.is_some())
    } else {
        r.status == expected.status && r.hardware.is_none() && r.hardware_failure.is_none()
    };
    let corrupt = !gates[0].passed
        || !gates[2].passed
        || !gates[3].passed
        || !gates[4].passed
        || !outcome_consistent
        || (r.hardware.is_some() && !gates[5].passed);
    if corrupt {
        // The received trace is evidence of the incident, never the trusted closure chain.
        let rejected = lgc;
        lgc = Lgc::new(input, previous);
        lgc.append(
            "SHADOW",
            "received_invalid_trace",
            json!({"untrusted_trace":rejected}),
        );
    }
    let spine = gates.iter().all(|g| g.passed);
    lgc.append(
        "SHADOW",
        "spine",
        json!({"gates":gates,"conjunction":spine}),
    );
    let selected = select(&r.assessments);
    let matrix = selected.is_some_and(|s| {
        s.missing_x.is_empty()
            && s.counterexamples.is_empty()
            && r.assessments
                .iter()
                .all(|a| a.context_sha256 == input.context_sha256())
    });
    gate(&mut gates, "Matrix.current_dossier", matrix);
    lgc.append("SHADOW","matrix",json!({"selected":r.selected,"passed":matrix,
        "negative_evidence_retained":r.assessments.iter().filter(|a|!a.counterexamples.is_empty()).count()}));
    let qualified = spine && matrix;
    let mut consumed = false;
    if qualified {
        let binding = serde_json::to_vec(&(input.input_sha256(), lgc.tip(), &r.selected))
            .map_err(|e| e.to_string())?;
        let h = ffi.issue(&binding)?;
        ffi.consume(h, &binding)?;
        consumed = true;
    }
    gate(&mut gates, "Judiciary.single_use_local_ffi", consumed);
    let bit = u8::from(gates.iter().all(|g| g.passed));
    lgc.append(
        "SHADOW",
        "judiciary",
        json!({"verification":bit,"all_required":true,"ffi_consumed":consumed}),
    );
    let status = if bit == 1 {
        "VerifiedAction"
    } else if corrupt {
        "FailClosedIntegrity"
    } else {
        &r.status
    }
    .to_string();
    witness(
        &mut lgc,
        "SHADOW",
        "outcome",
        json!({"status":status,"verification":bit}),
    );
    let seal = if !corrupt {
        Some(hash_value(
            "POC011_SEAL_500_V1",
            &(500, lgc.tip(), input.input_sha256()),
        ))
    } else {
        None
    };
    let mut questions: Vec<String> = input
        .law
        .gaps()
        .iter()
        .map(|g| {
            format!(
                "What is the applicable value for {g} in scope {}?",
                input.scope
            )
        })
        .collect();
    for a in &r.assessments {
        if !a.missing_x.is_empty() {
            questions.push(format!(
                "Provide current y evidence for candidate {} at x={:?} under y=a*x+b.",
                a.candidate_id, a.missing_x
            ));
        }
        for ce in &a.counterexamples {
            questions.push(format!("{ce}. What corrected candidate or evidence addresses this exact counterexample within the same law?"));
        }
    }
    if r.assessments.is_empty() && input.law.gaps().is_empty() {
        questions.push(
            "Which candidate and complete model table should LIGHT coordinate for this context?"
                .into(),
        );
    }
    if let Some(e) = &r.hardware_failure {
        questions.push(format!("Hardware execution lacks a valid receipt ({e}); what sufficient measured execution budget can be provided?"));
    }
    if corrupt {
        questions.push("Which current contract/trace was altered? Restore the exact bound input, order and evidence before retrying.".into());
    }
    let output = if bit == 1 {
        format!(
            "Verified bounded action: {}. Scope: {}. No claim beyond the finite model.",
            r.selected.as_deref().unwrap_or(""),
            input.scope
        )
    } else {
        format!("{status}: no action admitted. Current evidence and its gaps remain explicit.")
    };
    Ok(VerifiedClosure {
        report: Report {
            input: input.clone(),
            status,
            verification: bit,
            selected_candidate_sha256: r.selected.clone(),
            assessments: r.assessments.clone(),
            hardware: json!(r.hardware),
            gates,
            seal_500: seal,
            lgc,
            output,
            questions,
        },
    })
}

pub(crate) fn run_with_memory(
    input: &Input,
    previous: &str,
    ffi: &mut ActionGate,
    memory_refs: Vec<MemoryReference>,
) -> Result<VerifiedClosure, String> {
    let mut lgc = Lgc::new(input, previous);
    witness(
        &mut lgc,
        "LIGHT",
        "preconditions",
        json!({"input_sha256":input.input_sha256(),"law_gaps":input.law.gaps()}),
    );
    let light_input = pulse(&mut lgc, "LIGHT", input);
    let mut c = coordinate(light_input.request(), previous);
    c.memory_refs = memory_refs;
    lgc.append("LIGHT", "coordinate", json!(c));
    witness(
        &mut lgc,
        "QUANTUM",
        "preconditions",
        json!({"coordination_sha256":hash_value("POC011_COORDINATION_V1",&c),"law_gaps":input.law.gaps()}),
    );
    let quantum_input = pulse(&mut lgc, "QUANTUM", input);
    let r = reason(quantum_input.request());
    lgc.append("QUANTUM", "eliminate", json!(&r));
    witness(
        &mut lgc,
        "QUANTUM",
        "outcome",
        json!({"status":r.status,"assessment_sha256":hash_value("POC011_ASSESSMENTS_V1",&r.assessments),"hardware_failure":r.hardware_failure}),
    );
    let received_tip = lgc.tip();
    witness(
        &mut lgc,
        "SHADOW",
        "preconditions",
        json!({"received_tip":received_tip}),
    );
    let shadow_input = pulse(&mut lgc, "SHADOW", input);
    shadow(shadow_input.request(), previous, &c, &r, lgc, ffi)
}

#[cfg(test)]
pub(crate) fn run(
    input: &Input,
    previous: &str,
    ffi: &mut ActionGate,
) -> Result<VerifiedClosure, String> {
    run_with_memory(input, previous, ffi, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn good() -> Input {
        crate::demonstration_inputs().remove(3)
    }
    #[test]
    fn corrupt_hardware_receipt_loses_seal_and_produces_question() {
        let i = good();
        let c = coordinate(&i, "root");
        let mut r = reason(&i);
        r.hardware.as_mut().unwrap().payload_sha256 = "0".repeat(64);
        let l = trace_prefix(&i, "root", &c, &r);
        let p = shadow(&i, "root", &c, &r, l, &mut ActionGate::new())
            .unwrap()
            .into_report();
        assert_eq!(p.verification, 0);
        assert_eq!(p.status, "FailClosedIntegrity");
        assert!(p.seal_500.is_none());
        assert!(!p.questions.is_empty());
        assert!(p.lgc.verify());
    }
    #[test]
    fn missing_candidate_does_not_veto_another_fully_evidenced_candidate() {
        let mut i = good();
        i.candidates[1].a = 1;
        i.candidates[1].samples.clear();
        let p = run(&i, "root", &mut ActionGate::new())
            .unwrap()
            .into_report();
        assert_eq!(p.verification, 1);
        assert!(p.assessments.iter().any(|a| a.status == "MissingEvidence"));
        assert!(p
            .questions
            .iter()
            .any(|q| q.contains("Provide current y evidence")));
    }
    #[test]
    fn fabricated_quantum_gain_fails_even_with_rehashed_trace() {
        let i = good();
        let c = coordinate(&i, "root");
        let mut r = reason(&i);
        r.assessments[0].gain = Some(999999);
        let l = trace_prefix(&i, "root", &c, &r);
        let report = shadow(&i, "root", &c, &r, l, &mut ActionGate::new())
            .unwrap()
            .into_report();
        assert_eq!(report.verification, 0);
        assert_eq!(report.status, "FailClosedIntegrity");
        assert!(report.seal_500.is_none());
    }
    #[test]
    fn trace_byte_or_order_corruption_never_keeps_seal() {
        for order in [false, true] {
            let i = good();
            let c = coordinate(&i, "root");
            let r = reason(&i);
            let mut l = trace_prefix(&i, "root", &c, &r);
            if order {
                l.steps.swap(2, 5);
            } else {
                l.steps[2].payload = json!("altered");
            }
            let p = shadow(&i, "root", &c, &r, l, &mut ActionGate::new())
                .unwrap()
                .into_report();
            assert_eq!(p.verification, 0);
            assert_eq!(p.status, "FailClosedIntegrity");
            assert!(p.seal_500.is_none());
        }
    }
    #[test]
    fn prior_success_does_not_authorize_current_missing_evidence() {
        let mut i = good();
        let first = run(&i, "root", &mut ActionGate::new())
            .unwrap()
            .into_report();
        assert_eq!(first.verification, 1);
        i.candidates.iter_mut().for_each(|c| c.samples.clear());
        let second = run(&i, &first.lgc.tip(), &mut ActionGate::new())
            .unwrap()
            .into_report();
        assert_eq!(second.verification, 0);
        assert_eq!(second.status, "HoldMissingEvidence");
    }
    #[test]
    fn exact_counterexample_not_average_blocks_candidate() {
        let mut i = good();
        i.candidates = vec![i.candidates[0].clone()];
        i.candidates[0].samples[2].y += 1;
        let p = run(&i, "root", &mut ActionGate::new())
            .unwrap()
            .into_report();
        assert_eq!(p.status, "FailClosedCounterproof");
        assert_eq!(p.verification, 0);
        assert!(p.questions.iter().any(|q| q.contains("x=2")));
    }
    #[test]
    fn zero_physical_budget_cannot_be_replaced_by_positive_model() {
        let mut i = good();
        i.law.byte_budget = Some(0);
        let p = run(&i, "root", &mut ActionGate::new())
            .unwrap()
            .into_report();
        assert_eq!(p.verification, 0);
        assert_eq!(p.status, "HoldHardwareEvidence");
    }
    #[test]
    fn deterministic_collapse_ignores_candidate_enumeration_order() {
        let i = good();
        let mut reversed = i.clone();
        reversed.candidates.reverse();
        assert_eq!(
            logical_reasoning(&i).selected,
            logical_reasoning(&reversed).selected
        );
    }
}
