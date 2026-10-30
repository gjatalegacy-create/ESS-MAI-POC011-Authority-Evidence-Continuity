//! A bounded write/readback on process-owned memory, with measured elapsed time.
//!
//! Bytes are logical payload bytes, not allocator capacity or physical DRAM pages.
//! The deadline is checked cooperatively; this is not a hard real-time guarantee.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

const MAX_PAYLOAD_BYTES: u64 = 64 * 1024;
const VALUE_BYTES: u64 = 8;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HardwareReceipt {
    pub payload_sha256: String,
    pub requested_bytes: u64,
    pub actual_bytes: u64,
    pub elapsed_ns: u64,
    pub byte_budget: u64,
    pub time_budget_ns: u64,
    pub verified_readback: bool,
    pub physical_energy_joules: Option<u64>,
}

/// n = min(floor(B / C), 4). No slot is granted without its full byte cost.
pub(crate) fn affordable_slots(byte_budget: u64, bytes_per_slot: u64) -> Result<u8, String> {
    if bytes_per_slot == 0 {
        return Err("HW_SLOT_COST_ZERO".into());
    }
    Ok((byte_budget / bytes_per_slot).min(4) as u8)
}

fn payload_bytes(value_count: u64) -> Result<u64, String> {
    let bytes = value_count
        .checked_mul(VALUE_BYTES)
        .ok_or("HW_PAYLOAD_SIZE_OVERFLOW")?;
    if bytes == 0 {
        return Err("HW_EMPTY_INPUT".into());
    }
    if bytes > MAX_PAYLOAD_BYTES {
        return Err("HW_PAYLOAD_EXCEEDS_64_KIB".into());
    }
    Ok(bytes)
}

fn elapsed_within_budget(elapsed: Duration, time_budget_ns: u64) -> Result<u64, String> {
    if time_budget_ns == 0 {
        return Err("HW_TIME_BUDGET_ZERO".into());
    }
    let elapsed_ns = u64::try_from(elapsed.as_nanos()).map_err(|_| "HW_ELAPSED_OVERFLOW")?;
    if elapsed_ns > time_budget_ns {
        return Err(format!(
            "HW_DEADLINE_EXCEEDED: elapsed_ns={elapsed_ns}, budget_ns={time_budget_ns}"
        ));
    }
    Ok(elapsed_ns)
}

pub(crate) fn execute(
    values: &[i64],
    byte_budget: u64,
    time_budget_ns: u64,
) -> Result<HardwareReceipt, String> {
    if byte_budget == 0 {
        return Err("HW_BYTE_BUDGET_ZERO".into());
    }
    if time_budget_ns == 0 {
        return Err("HW_TIME_BUDGET_ZERO".into());
    }
    let count = u64::try_from(values.len()).map_err(|_| "HW_VALUE_COUNT_OVERFLOW")?;
    let requested_bytes = payload_bytes(count)?;
    if affordable_slots(byte_budget, requested_bytes)? == 0 {
        return Err("HW_PAYLOAD_EXCEEDS_BYTE_BUDGET".into());
    }
    let length = usize::try_from(requested_bytes).map_err(|_| "HW_ALLOCATION_SIZE_OVERFLOW")?;

    let start = Instant::now();
    let mut payload = Vec::<u8>::new();
    payload
        .try_reserve_exact(length)
        .map_err(|_| "HW_ALLOCATION_REFUSED")?;
    for value in values {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    elapsed_within_budget(start.elapsed(), time_budget_ns)?;

    // Read from the allocation itself. This observes no memory owned by others.
    let readback = std::hint::black_box(payload.as_slice());
    let verified_readback = readback.len() == length
        && readback
            .chunks_exact(8)
            .zip(values)
            .all(|(bytes, expected)| {
                let mut encoded = [0u8; 8];
                encoded.copy_from_slice(bytes);
                i64::from_le_bytes(encoded) == *expected
            });
    if !verified_readback {
        return Err("HW_READBACK_MISMATCH".into());
    }
    let payload_sha256 = format!("{:x}", Sha256::digest(readback));
    let actual_bytes = u64::try_from(payload.len()).map_err(|_| "HW_ACTUAL_SIZE_OVERFLOW")?;
    let elapsed_ns = elapsed_within_budget(start.elapsed(), time_budget_ns)?;

    Ok(HardwareReceipt {
        payload_sha256,
        requested_bytes,
        actual_bytes,
        elapsed_ns,
        byte_budget,
        time_budget_ns,
        verified_readback,
        physical_energy_joules: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_value_is_written_and_read_in_little_endian_order() {
        let values = [i64::MIN, -1, 0, 1, i64::MAX];
        let receipt = execute(&values, 40, 10_000_000_000).unwrap();
        let expected: Vec<u8> = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        assert_eq!(
            receipt.payload_sha256,
            format!("{:x}", Sha256::digest(&expected))
        );
        assert_eq!(receipt.requested_bytes, 40);
        assert_eq!(receipt.actual_bytes, 40);
        assert!(receipt.verified_readback);
        assert!(receipt.elapsed_ns <= receipt.time_budget_ns);
        assert_eq!(receipt.physical_energy_joules, None);
    }

    #[test]
    fn zero_budget_or_unfunded_payload_cannot_execute() {
        assert_eq!(execute(&[1], 0, 1).unwrap_err(), "HW_BYTE_BUDGET_ZERO");
        assert_eq!(
            execute(&[1], 7, 1).unwrap_err(),
            "HW_PAYLOAD_EXCEEDS_BYTE_BUDGET"
        );
        assert_eq!(execute(&[], 8, 1).unwrap_err(), "HW_EMPTY_INPUT");
    }

    #[test]
    fn payload_limit_and_overflow_fail_before_allocation() {
        assert_eq!(payload_bytes(8192).unwrap(), MAX_PAYLOAD_BYTES);
        assert_eq!(
            payload_bytes(8193).unwrap_err(),
            "HW_PAYLOAD_EXCEEDS_64_KIB"
        );
        assert_eq!(
            payload_bytes(u64::MAX).unwrap_err(),
            "HW_PAYLOAD_SIZE_OVERFLOW"
        );
    }

    #[test]
    fn deadline_boundaries_are_exact_and_invalid_deadlines_fail() {
        assert_eq!(execute(&[1], 8, 0).unwrap_err(), "HW_TIME_BUDGET_ZERO");
        assert_eq!(
            elapsed_within_budget(Duration::from_nanos(5), 5).unwrap(),
            5
        );
        assert!(elapsed_within_budget(Duration::from_nanos(6), 5)
            .unwrap_err()
            .starts_with("HW_DEADLINE_EXCEEDED:"));
        assert_eq!(
            elapsed_within_budget(Duration::from_secs(u64::MAX), u64::MAX).unwrap_err(),
            "HW_ELAPSED_OVERFLOW"
        );
    }

    #[test]
    fn slots_preserve_the_source_budget_law_without_unfunded_minimum() {
        assert_eq!(affordable_slots(0, 8).unwrap(), 0);
        assert_eq!(affordable_slots(7, 8).unwrap(), 0);
        assert_eq!(affordable_slots(8, 8).unwrap(), 1);
        assert_eq!(affordable_slots(31, 8).unwrap(), 3);
        assert_eq!(affordable_slots(32, 8).unwrap(), 4);
        assert_eq!(affordable_slots(u64::MAX, 8).unwrap(), 4);
        assert_eq!(affordable_slots(8, 0).unwrap_err(), "HW_SLOT_COST_ZERO");
    }
}
