//! A bounded, process-local action boundary around the extracted ESS-MAI gate.
//! Binding bytes are checked before consuming the private one-use token.

#[rustfmt::skip]
#[path = "source_ffi.rs"]
mod source_ffi;

use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) type Handle = source_ffi::CapHandle;
const MAX_ACTIONS: usize = 1024;
const MAX_BINDING_BYTES: usize = 4096;
static NEXT_GATE: AtomicU64 = AtomicU64::new(0);

/// One session owns this gate for its entire run, including all five cycles.
/// No public API mints capabilities; the session supplies exact contract bytes.
pub(crate) struct ActionGate {
    source: source_ffi::SovereignGate,
    nonce: Option<u64>,
    bindings: Vec<Box<[u8]>>,
}

fn allocate_gate(sequence: &AtomicU64) -> Option<u64> {
    let mut current = sequence.load(Ordering::Relaxed);
    loop {
        let next = current.checked_add(1)?;
        match sequence.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Some(next),
            Err(actual) => current = actual,
        }
    }
}

impl ActionGate {
    pub(crate) fn new() -> Self {
        Self {
            source: source_ffi::SovereignGate::new(),
            nonce: allocate_gate(&NEXT_GATE),
            bindings: Vec::new(),
        }
    }

    pub(crate) fn issue(&mut self, binding: &[u8]) -> Result<Handle, String> {
        let nonce = self.nonce.ok_or("FFI_GATE_SEQUENCE_EXHAUSTED")?;
        if binding.is_empty() || binding.len() > MAX_BINDING_BYTES {
            return Err("FFI_BINDING_LIMIT".into());
        }
        if self.bindings.len() >= MAX_ACTIONS {
            return Err("FFI_ACTION_LIMIT".into());
        }
        // The native generation counter cannot overflow within this fixed cap.
        let owned = binding.to_vec().into_boxed_slice();
        let handle = self.source.issue(nonce);
        self.bindings.push(owned);
        Ok(handle)
    }

    pub(crate) fn consume(&self, handle: Handle, binding: &[u8]) -> Result<(), String> {
        let index = usize::try_from(handle.gen).map_err(|_| "FFI_HANDLE_NOT_FOUND")?;
        let expected = self.bindings.get(index).ok_or("FFI_HANDLE_NOT_FOUND")?;
        if expected.as_ref() != binding {
            return Err("FFI_BINDING_MISMATCH".into());
        }
        // The non-Clone/non-Copy LgcToken is consumed in this private boundary.
        self.source
            .validate(handle)
            .map(|_token| ())
            .map_err(|error| format!("FFI_SOURCE_{error:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_gate_allocations_are_unique_and_contiguous() {
        let sequence = AtomicU64::new(0);
        let barrier = std::sync::Barrier::new(32);
        let mut allocated = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..32)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        (0..64)
                            .map(|_| allocate_gate(&sequence).unwrap())
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|worker| worker.join().unwrap())
                .collect::<Vec<_>>()
        });
        allocated.sort_unstable();
        assert_eq!(allocated, (1..=2048).collect::<Vec<u64>>());
        assert_eq!(sequence.load(Ordering::Relaxed), 2048);
    }

    #[test]
    fn concurrent_gate_exhaustion_has_one_final_allocation_and_never_wraps() {
        let sequence = AtomicU64::new(u64::MAX - 1);
        let barrier = std::sync::Barrier::new(32);
        let outcomes = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..32)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        allocate_gate(&sequence)
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(outcomes.iter().filter(|v| **v == Some(u64::MAX)).count(), 1);
        assert_eq!(outcomes.iter().filter(|v| v.is_none()).count(), 31);
        assert_eq!(sequence.load(Ordering::Relaxed), u64::MAX);
        assert_eq!(allocate_gate(&sequence), None);
    }

    #[test]
    fn wrong_nonce_does_not_burn_legitimate_handle() {
        let mut gate = ActionGate::new();
        let handle = gate.issue(b"session/cycle-1/light").unwrap();
        let mut changed = handle;
        changed.nonce ^= 1;
        assert!(gate.consume(changed, b"session/cycle-1/light").is_err());
        assert!(gate.consume(handle, b"session/cycle-1/light").is_ok());
    }

    #[test]
    fn changed_binding_does_not_burn_legitimate_handle() {
        let mut gate = ActionGate::new();
        let handle = gate.issue(b"session/cycle-1/quantum").unwrap();
        assert!(gate.consume(handle, b"session/cycle-2/quantum").is_err());
        assert!(gate.consume(handle, b"session/cycle-1/quantum").is_ok());
    }

    #[test]
    fn copied_handles_consume_exactly_once() {
        let mut gate = ActionGate::new();
        let handle = gate.issue(b"cycle-1/shadow").unwrap();
        let copies = [handle; 8];
        assert_eq!(
            copies
                .into_iter()
                .filter(|h| gate.consume(*h, b"cycle-1/shadow").is_ok())
                .count(),
            1
        );
    }

    #[test]
    fn concurrent_32_copies_have_one_winner() {
        let mut gate = ActionGate::new();
        let handle = gate.issue(b"cycle-1/shadow").unwrap();
        let winners = std::thread::scope(|scope| {
            let gate = &gate;
            let workers: Vec<_> = (0..32)
                .map(|_| {
                    scope
                        .spawn(move || usize::from(gate.consume(handle, b"cycle-1/shadow").is_ok()))
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .sum::<usize>()
        });
        assert_eq!(winners, 1);
    }

    #[test]
    fn another_session_gate_cannot_consume_copied_handle() {
        let mut first = ActionGate::new();
        let mut second = ActionGate::new();
        let one = first.issue(b"same-bytes").unwrap();
        let two = second.issue(b"same-bytes").unwrap();
        assert!(second.consume(one, b"same-bytes").is_err());
        assert!(second.consume(two, b"same-bytes").is_ok());
        assert!(first.consume(one, b"same-bytes").is_ok());
    }

    #[test]
    fn issuance_is_bounded_and_exhaustion_does_not_wrap() {
        let mut gate = ActionGate::new();
        assert!(gate.issue(b"").is_err());
        assert!(gate.issue(&vec![0; MAX_BINDING_BYTES + 1]).is_err());
        for _ in 0..MAX_ACTIONS {
            let handle = gate.issue(b"action").unwrap();
            gate.consume(handle, b"action").unwrap();
        }
        assert!(gate.issue(b"action").is_err());
        let sequence = AtomicU64::new(u64::MAX - 1);
        assert_eq!(allocate_gate(&sequence), Some(u64::MAX));
        assert_eq!(allocate_gate(&sequence), None);
        assert_eq!(sequence.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn consumed_token_does_not_cross_wrapper_interface() {
        let mut gate = ActionGate::new();
        let handle = gate.issue(b"exact-contract").unwrap();
        let output: Result<(), String> = gate.consume(handle, b"exact-contract");
        assert_eq!(output, Ok(()));
        assert_eq!(
            std::mem::size_of::<Handle>(),
            2 * std::mem::size_of::<u64>()
        );
        assert_eq!(std::mem::align_of::<Handle>(), std::mem::align_of::<u64>());
    }

    // Internal ABI adapter only: callers must supply live pointers for the call.
    // This exercises the actual C calling convention, not an external C compiler.
    unsafe extern "C" fn consume_ffi_probe(
        gate: *const ActionGate,
        handle: Handle,
        bytes: *const u8,
        len: usize,
    ) -> i32 {
        if gate.is_null() || bytes.is_null() || len == 0 || len > MAX_BINDING_BYTES {
            return -1;
        }
        // SAFETY: the test passes its live gate and immutable slice for this call.
        let gate = unsafe { &*gate };
        let binding = unsafe { std::slice::from_raw_parts(bytes, len) };
        match gate.consume(handle, binding) {
            Ok(()) => 1,
            Err(_) => 0,
        }
    }

    #[test]
    fn c_abi_copied_handle_probe_accepts_one_of_eight() {
        let mut gate = ActionGate::new();
        let binding = b"session/cycle-1/shadow";
        let handle = gate.issue(binding).unwrap();
        let call: unsafe extern "C" fn(*const ActionGate, Handle, *const u8, usize) -> i32 =
            consume_ffi_probe;
        let outcomes: Vec<_> = [handle; 8]
            .into_iter()
            .map(|copy| {
                // SAFETY: all three references remain live and unmodified through call.
                unsafe { call(&gate, copy, binding.as_ptr(), binding.len()) }
            })
            .collect();
        assert_eq!(outcomes, vec![1, 0, 0, 0, 0, 0, 0, 0]);
    }
}
