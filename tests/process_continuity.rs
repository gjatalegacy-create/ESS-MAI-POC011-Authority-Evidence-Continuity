use poc011::{demonstration_inputs, CycleOutput};
use std::process::Command;

fn directory(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "poc011-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}
fn invoke(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_poc011"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn five_independent_processes_share_context_without_inheriting_verdicts() {
    let dir = directory("processes");
    let wal = dir.join("memory.wal");
    let w = wal.to_str().unwrap();
    assert!(invoke(&["init", w]).status.success());
    let mut bits = vec![];
    let mut receipts = vec![];
    for input in demonstration_inputs() {
        let path = dir.join(format!("{}.json", input.cycle));
        std::fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
        let output = invoke(&["cycle", w, path.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let cycle: CycleOutput = serde_json::from_slice(&output.stdout).unwrap();
        bits.push(cycle.report.verification);
        let refs = &cycle.report.lgc.steps[2].payload["memory_refs"];
        assert_eq!(refs.as_array().unwrap().len(), (input.cycle - 1) as usize);
        for (i, reference) in refs.as_array().unwrap().iter().enumerate() {
            assert_eq!(reference["receipt_sha256"], receipts[i]);
            assert_eq!(reference["context_sha256"], input.context_sha256());
        }
        receipts.push(serde_json::Value::String(cycle.commit.receipt_sha256));
    }
    assert_eq!(bits, [0, 0, 0, 1, 1]);
    let observed = invoke(&["inspect", w]);
    assert!(observed.status.success());
    let memory: serde_json::Value = serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(memory["effect_count"], 2);
    assert_eq!(
        memory["records"][2]["report"]["assessments"][0]["status"],
        "EliminatedCounterproof"
    );
    let before = std::fs::read(&wal).unwrap();
    assert!(!invoke(&["cycle", w, dir.join("5.json").to_str().unwrap()])
        .status
        .success());
    assert_eq!(before, std::fs::read(&wal).unwrap());
    for entry in std::fs::read_dir(&dir).unwrap() {
        std::fs::remove_file(entry.unwrap().path()).unwrap();
    }
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn oversized_or_duplicate_field_inputs_do_not_touch_memory() {
    let dir = directory("invalid");
    let wal = dir.join("memory.wal");
    let w = wal.to_str().unwrap();
    assert!(invoke(&["init", w]).status.success());
    let before = std::fs::read(&wal).unwrap();
    let input = dir.join("input.json");
    std::fs::write(&input, vec![b' '; 65537]).unwrap();
    assert!(!invoke(&["cycle", w, input.to_str().unwrap()])
        .status
        .success());
    let canonical = serde_json::to_string(&demonstration_inputs()[0]).unwrap();
    let duplicate = canonical.replacen("{", "{\"cycle\":1,", 1);
    std::fs::write(&input, duplicate).unwrap();
    assert!(!invoke(&["cycle", w, input.to_str().unwrap()])
        .status
        .success());
    assert_eq!(before, std::fs::read(&wal).unwrap());
    std::fs::remove_file(input).unwrap();
    std::fs::remove_file(wal).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
