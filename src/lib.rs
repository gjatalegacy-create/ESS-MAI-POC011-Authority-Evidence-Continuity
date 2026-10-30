//! A bounded extraction/adaptation experiment, not the full ESS-MAI runtime.
mod authority;
mod domain;
mod ffi;
mod hardware;
mod storage;
mod trace;
#[rustfmt::skip]
mod zero_noi;

pub use authority::{Gate, Report};
pub use domain::{demonstration_inputs, Candidate, Input, Law, Sample};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
pub use storage::{inspect, Commit};

pub(crate) fn hash_parts(domain: &str, parts: &[&[u8]]) -> String {
    let mut h = Sha256::new();
    for part in std::iter::once(domain.as_bytes()).chain(parts.iter().copied()) {
        h.update((part.len() as u64).to_le_bytes());
        h.update(part);
    }
    format!("{:x}", h.finalize())
}
pub(crate) fn hash_value<T: Serialize>(domain: &str, value: &T) -> String {
    // All internal types contain bounded integers, strings and finite structures.
    let bytes = serde_json::to_vec(value).expect("bounded internal serialization");
    hash_parts(domain, &[&bytes])
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CycleOutput {
    pub report: Report,
    pub commit: Commit,
    pub gcl_restart_after_durable_commit: bool,
    pub next_cycle: u64,
}

pub struct Engine {
    store: storage::Store,
    ffi: ffi::ActionGate,
    in_flight: bool,
}
impl Engine {
    pub fn create(path: &Path) -> Result<Self, String> {
        Ok(Self {
            store: storage::Store::create(path)?,
            ffi: ffi::ActionGate::new(),
            in_flight: false,
        })
    }
    pub fn open(path: &Path) -> Result<Self, String> {
        Ok(Self {
            store: storage::Store::open(path)?,
            ffi: ffi::ActionGate::new(),
            in_flight: false,
        })
    }
    pub fn next_cycle(&self) -> u64 {
        self.store.next_cycle()
    }
    pub fn process(&mut self, input: &Input) -> Result<CycleOutput, String> {
        input.validate_shape()?;
        if self.in_flight {
            return Err("RecoveryRequired: previous cycle did not reach durable closure".into());
        }
        if input.cycle != self.next_cycle() {
            return Err("cycle must equal next durable sequence".into());
        }
        self.in_flight = true;
        let memory_refs = self
            .store
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.report.input.context_sha256() == input.context_sha256())
            .map(|(index, r)| authority::MemoryReference {
                cycle: r.sequence,
                context_sha256: r.report.input.context_sha256(),
                receipt_sha256: self
                    .store
                    .records
                    .get(index + 1)
                    .map(|next| next.previous_receipt_sha256.clone())
                    .unwrap_or_else(|| self.store.head().into()),
                negative_candidate_sha256: r
                    .report
                    .assessments
                    .iter()
                    .filter(|a| a.status == "EliminatedCounterproof")
                    .map(|a| a.candidate_sha256.clone())
                    .collect(),
                positive_candidate_sha256: if r.report.verification == 1 {
                    r.report.selected_candidate_sha256.iter().cloned().collect()
                } else {
                    vec![]
                },
                question_count: r.report.questions.len(),
            })
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let closure =
            authority::run_with_memory(input, self.store.head(), &mut self.ffi, memory_refs)?;
        let (report, commit) = self.store.commit(closure)?;
        // iZ = current output + exact gap/counterexample questions, delivered only after commit.
        // GCL continuity resets transient work only; gate and persistent map survive.
        self.in_flight = false;
        Ok(CycleOutput {
            report,
            commit,
            gcl_restart_after_durable_commit: true,
            next_cycle: self.next_cycle(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "poc011-{}-{}-{}.wal",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
    #[test]
    fn five_cycles_current_proof_and_durable_restart() {
        let p = path();
        let mut e = Engine::create(&p).unwrap();
        let inputs = demonstration_inputs();
        let mut bits = vec![];
        for i in &inputs {
            let out = e.process(i).unwrap();
            bits.push(out.report.verification);
            assert!(
                out.commit.durable
                    && out.commit.readback_verified
                    && out.gcl_restart_after_durable_commit
            );
            assert!(out.report.lgc.verify());
            assert_eq!(out.next_cycle, i.cycle + 1);
        }
        assert_eq!(bits, vec![0, 0, 0, 1, 1]);
        drop(e);
        let observed = inspect(&p).unwrap();
        assert_eq!(observed["effect_count"], 2);
        assert_eq!(Engine::open(&p).unwrap().next_cycle(), 6);
        std::fs::remove_file(p).unwrap();
    }
    #[test]
    fn same_action_same_values_has_one_effect_across_process_reopen() {
        let p = path();
        let mut i = demonstration_inputs().remove(3);
        i.cycle = 1;
        let mut e = Engine::create(&p).unwrap();
        assert!(e.process(&i).unwrap().commit.new_effect);
        drop(e);
        let mut e = Engine::open(&p).unwrap();
        i.cycle = 2;
        let out = e.process(&i).unwrap();
        assert_eq!(out.report.verification, 1);
        assert!(!out.commit.new_effect);
        assert_eq!(out.commit.effect_count, 1);
        drop(e);
        std::fs::remove_file(p).unwrap();
    }
    #[test]
    fn conflicting_effect_never_resets_or_overwrites() {
        let p = path();
        let mut i = demonstration_inputs().remove(3);
        i.cycle = 1;
        let mut e = Engine::create(&p).unwrap();
        e.process(&i).unwrap();
        drop(e);
        let bytes = std::fs::read(&p).unwrap();
        let mut e = Engine::open(&p).unwrap();
        let mut j = demonstration_inputs().remove(4);
        j.cycle = 2;
        j.action_id = i.action_id;
        assert!(e.process(&j).unwrap_err().contains("conflicting replay"));
        assert_eq!(e.next_cycle(), 2);
        assert!(e.process(&j).unwrap_err().contains("RecoveryRequired"));
        drop(e);
        assert_eq!(bytes, std::fs::read(&p).unwrap());
        std::fs::remove_file(p).unwrap();
    }
    #[test]
    fn altered_or_truncated_memory_fails_closed_on_reopen() {
        for truncate in [false, true] {
            let p = path();
            let mut e = Engine::create(&p).unwrap();
            e.process(&demonstration_inputs()[0]).unwrap();
            drop(e);
            let mut b = std::fs::read(&p).unwrap();
            if truncate {
                b.pop();
            } else {
                *b.last_mut().unwrap() ^= 1;
            }
            std::fs::write(&p, &b).unwrap();
            assert!(Engine::open(&p).is_err());
            assert_eq!(std::fs::read(&p).unwrap(), b);
            std::fs::remove_file(p).unwrap();
        }
    }
    #[test]
    fn unknown_json_fields_and_duplicate_candidates_are_not_silent() {
        let mut v = serde_json::to_value(&demonstration_inputs()[0]).unwrap();
        v["force_accept"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Input>(v).is_err());
        let mut i = demonstration_inputs().remove(0);
        i.candidates.push(i.candidates[0].clone());
        assert!(i.validate_shape().is_err());
    }
    #[test]
    fn exclusive_store_lock_refuses_second_writer() {
        let p = path();
        let first = Engine::create(&p).unwrap();
        assert!(Engine::open(&p).is_err());
        drop(first);
        std::fs::remove_file(p).unwrap();
    }
}
