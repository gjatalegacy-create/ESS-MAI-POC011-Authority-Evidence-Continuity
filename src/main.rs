use poc011::{demonstration_inputs, inspect, Commit, Engine, Input};
use std::io::{Read, Write};
use std::path::Path;

const INPUT_LIMIT: u64 = 65536;

#[derive(Debug, serde::Serialize)]
struct CliFailure {
    status: &'static str,
    error: String,
    commit_status: &'static str,
    last_known_commit: Option<Commit>,
    retry_automatically: bool,
}
impl From<String> for CliFailure {
    fn from(error: String) -> Self {
        Self {
            status: "CommandFailed",
            error,
            commit_status: "not_assessed",
            last_known_commit: None,
            retry_automatically: false,
        }
    }
}
impl CliFailure {
    fn processing(error: String) -> Self {
        Self {
            status: "ProcessingFailed",
            error,
            commit_status: "uncertain_reopen_and_inspect_before_retry",
            last_known_commit: None,
            retry_automatically: false,
        }
    }
    fn delivery(error: String, commit: Option<&Commit>) -> Self {
        Self {
            status: "OutputDeliveryFailed",
            error,
            commit_status: if commit.is_some() {
                "durable"
            } else {
                "not_assessed"
            },
            last_known_commit: commit.cloned(),
            retry_automatically: false,
        }
    }
}

fn emit_to<T: serde::Serialize>(mut out: impl Write, value: &T) -> Result<(), String> {
    serde_json::to_writer(&mut out, value).map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}
fn emit<T: serde::Serialize>(value: &T) -> Result<(), String> {
    emit_to(std::io::stdout().lock(), value)
}
fn read_input(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(INPUT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > INPUT_LIMIT {
        return Err("input exceeds 64 KiB".into());
    }
    Ok(bytes)
}
fn run() -> Result<(), CliFailure> {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("demo") if a.len() == 3 => {
            let dir = Path::new(&a[2]);
            std::fs::create_dir(dir).map_err(|e| e.to_string())?;
            let wal = dir.join("memory.wal");
            let mut engine = Engine::create(&wal)?;
            let mut last_commit = None;
            for input in demonstration_inputs() {
                std::fs::write(
                    dir.join(format!("input-{}.json", input.cycle)),
                    serde_json::to_vec_pretty(&input).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                let out = engine.process(&input).map_err(CliFailure::processing)?;
                let bytes = serde_json::to_vec_pretty(&out)
                    .map_err(|e| CliFailure::delivery(e.to_string(), Some(&out.commit)))?;
                std::fs::write(dir.join(format!("cycle-{}.json", input.cycle)), bytes)
                    .map_err(|e| CliFailure::delivery(e.to_string(), Some(&out.commit)))?;
                emit(&out).map_err(|e| CliFailure::delivery(e, Some(&out.commit)))?;
                last_commit = Some(out.commit);
            }
            drop(engine);
            let inspected = inspect(&wal)?;
            emit(&inspected).map_err(|e| CliFailure::delivery(e, last_commit.as_ref()))
        }
        Some("init") if a.len() == 3 => {
            drop(Engine::create(Path::new(&a[2]))?);
            emit(&serde_json::json!({"initialized":true}))
                .map_err(|e| CliFailure::delivery(e, None))
        }
        Some("cycle") if a.len() == 4 => {
            let file = std::fs::File::open(&a[3]).map_err(|e| e.to_string())?;
            let bytes = read_input(file)?;
            let input: Input = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let mut engine = Engine::open(Path::new(&a[2]))?;
            let out = engine.process(&input).map_err(CliFailure::processing)?;
            emit(&out).map_err(|e| CliFailure::delivery(e, Some(&out.commit)))
        }
        Some("inspect") if a.len() == 3 => {
            emit(&inspect(Path::new(&a[2]))?).map_err(|e| CliFailure::delivery(e, None))
        }
        _ => Err(CliFailure::from(
            "usage: poc011 demo NEW_DIRECTORY | init WAL | cycle WAL INPUT.json | inspect WAL"
                .to_string(),
        )),
    }
}
fn main() {
    if let Err(error) = run() {
        // Losing the diagnostic sink cannot undo a commit or create a second panic.
        let _ = emit_to(std::io::stderr().lock(), &error);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_input_is_read_only_through_limit_plus_one() {
        let bytes = vec![b'x'; INPUT_LIMIT as usize + 1024];
        let mut reader = std::io::Cursor::new(bytes);
        assert!(read_input(&mut reader).is_err());
        assert_eq!(reader.position(), INPUT_LIMIT + 1);
        assert_eq!(read_input(&b"{}"[..]).unwrap(), b"{}");
    }

    #[test]
    fn broken_output_keeps_known_commit_and_does_not_claim_refusal() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let commit = Commit {
            sequence: 4,
            receipt_sha256: "a".repeat(64),
            effect_count: 1,
            new_effect: true,
            durable: true,
            readback_verified: true,
        };
        let failure = CliFailure::delivery(emit_to(Broken, &commit).unwrap_err(), Some(&commit));
        assert_eq!(failure.status, "OutputDeliveryFailed");
        assert_eq!(failure.commit_status, "durable");
        assert_eq!(
            failure.last_known_commit.unwrap().receipt_sha256,
            commit.receipt_sha256
        );
        assert!(!failure.retry_automatically);
    }

    #[test]
    fn processing_failure_does_not_invent_rollback() {
        let failure = CliFailure::processing("sync acknowledgment lost".into());
        assert_eq!(
            failure.commit_status,
            "uncertain_reopen_and_inspect_before_retry"
        );
        assert!(!failure.retry_automatically);
    }
}
