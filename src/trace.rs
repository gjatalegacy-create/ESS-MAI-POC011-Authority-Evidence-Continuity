use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub sequence: u64,
    pub role: String,
    pub operation: String,
    pub payload: Value,
    pub previous_sha256: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Lgc {
    pub input_sha256: String,
    pub context_sha256: String,
    pub previous_cycle_sha256: String,
    pub steps: Vec<Step>,
}

impl Lgc {
    pub(crate) fn new(input: &crate::Input, previous: &str) -> Self {
        Self {
            input_sha256: input.input_sha256(),
            context_sha256: input.context_sha256(),
            previous_cycle_sha256: previous.into(),
            steps: Vec::new(),
        }
    }
    fn seed(&self) -> String {
        crate::hash_parts(
            "POC011_LGC_START_V1",
            &[
                self.input_sha256.as_bytes(),
                self.context_sha256.as_bytes(),
                self.previous_cycle_sha256.as_bytes(),
            ],
        )
    }
    pub fn tip(&self) -> String {
        self.steps
            .last()
            .map(|s| s.sha256.clone())
            .unwrap_or_else(|| self.seed())
    }
    pub(crate) fn append(&mut self, role: &str, operation: &str, payload: Value) {
        let mut s = Step {
            sequence: self.steps.len() as u64,
            role: role.into(),
            operation: operation.into(),
            payload,
            previous_sha256: self.tip(),
            sha256: String::new(),
        };
        s.sha256 = step_hash(&s);
        self.steps.push(s);
    }
    pub fn verify(&self) -> bool {
        let mut previous = self.seed();
        for (i, s) in self.steps.iter().enumerate() {
            if s.sequence != i as u64 || s.previous_sha256 != previous || s.sha256 != step_hash(s) {
                return false;
            }
            previous = s.sha256.clone();
        }
        true
    }
}
fn step_hash(s: &Step) -> String {
    crate::hash_value(
        "POC011_LGC_STEP_V1",
        &(
            s.sequence,
            &s.role,
            &s.operation,
            &s.payload,
            &s.previous_sha256,
        ),
    )
}
