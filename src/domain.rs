use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SCOPE: &str = "poc011/affine-admission";
pub const DOMAIN: [i64; 5] = [0, 1, 2, 3, 4];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Law {
    pub jurisdiction: Option<String>,
    pub risk_limit: Option<u64>,
    pub byte_budget: Option<u64>,
    pub time_budget_ns: Option<u64>,
}

impl Law {
    pub(crate) fn gaps(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.jurisdiction.is_none() {
            out.push("law.jurisdiction".into());
        }
        if self.risk_limit.is_none() {
            out.push("law.risk_limit".into());
        }
        if self.byte_budget.is_none() {
            out.push("law.byte_budget".into());
        }
        if self.time_budget_ns.is_none() {
            out.push("law.time_budget_ns".into());
        }
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub x: i64,
    pub y: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub a: i64,
    pub b: i64,
    pub samples: Vec<Sample>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub cycle: u64,
    pub context: String,
    pub scope: String,
    pub action_id: String,
    pub question: String,
    pub law: Law,
    pub candidates: Vec<Candidate>,
}

impl Input {
    pub(crate) fn validate_shape(&self) -> Result<(), String> {
        if self.cycle == 0 || self.cycle > 1024 || self.scope != SCOPE {
            return Err("invalid cycle or jurisdiction".into());
        }
        for s in [&self.context, &self.action_id, &self.question] {
            if s.is_empty() || s.len() > 2048 {
                return Err("invalid bounded text".into());
            }
        }
        if self.law.jurisdiction.as_deref().is_some_and(|s| s != SCOPE)
            || self.law.risk_limit.is_some_and(|v| v > 10_000_000)
            || self.law.byte_budget.is_some_and(|v| v > 65536)
            || self.law.time_budget_ns.is_some_and(|v| v > 60_000_000_000)
            || self.candidates.len() > 4
        {
            return Err("law or candidate bounds invalid".into());
        }
        let mut ids = BTreeSet::new();
        let mut models = BTreeSet::new();
        for c in &self.candidates {
            if c.id.is_empty()
                || c.id.len() > 128
                || !ids.insert(&c.id)
                || !models.insert((c.a, c.b))
                || c.a.unsigned_abs() > 1_000_000
                || c.b.unsigned_abs() > 1_000_000
                || c.samples.len() > DOMAIN.len()
            {
                return Err("duplicate or invalid candidate".into());
            }
            let mut xs = BTreeSet::new();
            for s in &c.samples {
                if !DOMAIN.contains(&s.x) || !xs.insert(s.x) || s.y.unsigned_abs() > 10_000_000 {
                    return Err("duplicate or invalid evidence coordinate".into());
                }
            }
        }
        Ok(())
    }
    pub fn context_sha256(&self) -> String {
        crate::hash_parts(
            "POC011_CONTEXT_V1",
            &[self.context.as_bytes(), self.scope.as_bytes()],
        )
    }
    pub fn input_sha256(&self) -> String {
        crate::hash_value("POC011_INPUT_V1", self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub candidate_id: String,
    pub candidate_sha256: String,
    pub context_sha256: String,
    pub status: String,
    pub missing_x: Vec<i64>,
    pub counterexamples: Vec<String>,
    pub values: Vec<i64>,
    pub risk: Option<u64>,
    pub gain: Option<i64>,
}

/// Finite model only: y=a*x+b on D={0,1,2,3,4}. Missing evidence is not refutation.
pub(crate) fn assess(input: &Input) -> Vec<Assessment> {
    input
        .candidates
        .iter()
        .map(|c| {
            let mut a = Assessment {
                candidate_id: c.id.clone(),
                candidate_sha256: crate::hash_value("POC011_CANDIDATE_V1", c),
                context_sha256: input.context_sha256(),
                status: "MissingEvidence".into(),
                missing_x: Vec::new(),
                counterexamples: Vec::new(),
                values: Vec::new(),
                risk: None,
                gain: None,
            };
            for x in DOMAIN {
                let expected = c.a * x + c.b; // validated bounds prove |y| <= 5,000,000
                match c.samples.iter().find(|s| s.x == x) {
                    None => a.missing_x.push(x),
                    Some(s) if s.y != expected => a.counterexamples.push(format!(
                        "candidate {}: x={x}, supplied y={}, model y={expected}",
                        c.id, s.y
                    )),
                    Some(_) => {}
                }
                a.values.push(expected);
            }
            let risk = a.values.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
            a.risk = Some(risk);
            a.gain = Some(a.values.iter().sum());
            if let Some(limit) = input.law.risk_limit {
                for (x, y) in DOMAIN.into_iter().zip(&a.values) {
                    if y.unsigned_abs() > limit {
                        a.counterexamples.push(format!(
                            "candidate {}: x={x}, |y|={} > risk_limit={limit}",
                            c.id,
                            y.unsigned_abs()
                        ));
                    }
                }
            }
            a.status = if !a.counterexamples.is_empty() {
                "EliminatedCounterproof"
            } else if !a.missing_x.is_empty() {
                "MissingEvidence"
            } else {
                "Eligible"
            }
            .into();
            a
        })
        .collect()
}

/// No stochastic collapse and no compensation for a hard law violation.
pub(crate) fn select(assessments: &[Assessment]) -> Option<&Assessment> {
    assessments
        .iter()
        .filter(|a| a.status == "Eligible")
        .min_by(|a, b| {
            b.gain
                .cmp(&a.gain)
                .then(a.risk.cmp(&b.risk))
                .then(a.candidate_sha256.cmp(&b.candidate_sha256))
        })
}

pub fn demonstration_inputs() -> Vec<Input> {
    fn candidate(id: &str, a: i64, b: i64) -> Candidate {
        Candidate {
            id: id.into(),
            a,
            b,
            samples: DOMAIN
                .into_iter()
                .map(|x| Sample { x, y: a * x + b })
                .collect(),
        }
    }
    let base = Input {
        cycle: 1,
        context: "Bounded affine compute admission: a five-turn conversation".into(),
        scope: SCOPE.into(),
        action_id: "model-A".into(),
        question: "Admit the highest-gain evidenced model within the risk bound.".into(),
        law: Law {
            jurisdiction: Some(SCOPE.into()),
            risk_limit: None,
            byte_budget: Some(4096),
            time_budget_ns: Some(1_000_000_000),
        },
        candidates: vec![candidate("safe", 2, 1)],
    };
    let mut two = base.clone();
    two.cycle = 2;
    two.law.risk_limit = Some(10);
    two.candidates[0].samples.clear();
    two.question = "The risk limit is 10; what evidence is still missing?".into();
    let mut three = two.clone();
    three.cycle = 3;
    three.candidates = vec![candidate("over-budget-model", 4, 1)];
    three.question = "Here is a complete model table; test its safety bound.".into();
    let mut four = three.clone();
    four.cycle = 4;
    four.candidates = vec![
        candidate("safe", 2, 1),
        candidate("over-budget-model", 4, 1),
    ];
    four.question = "Retain the counterexample and evaluate this corrected candidate.".into();
    let mut five = four.clone();
    five.cycle = 5;
    five.action_id = "model-B".into();
    five.candidates = vec![
        candidate("stronger-safe", 2, 2),
        candidate("challenger", 4, 1),
    ];
    five.question =
        "Evaluate fresh evidence for a stronger candidate; prior success is not permission.".into();
    vec![base, two, three, four, five]
}
