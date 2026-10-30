use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
pub struct LgcToken(PhantomData<*const ()>);

impl LgcToken {
    fn new() -> Self {
        LgcToken(PhantomData)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SovErr {
    NonceMismatch { expected: u64, actual: u64 },
    AlreadyConsumed,
    NotFound,
}
struct CapSlot {
    live: AtomicBool,
    expected_nonce: u64,
}

impl CapSlot {
    fn new(expected_nonce: u64) -> Self {
        Self {
            live: AtomicBool::new(true),
            expected_nonce,
        }
    }

    /// Konsumon qelizën. CAS(true→false): vetëm hera e parë kalon.
    /// Çdo thirrje tjetër → false (already consumed). Hardware-atomik.
    fn consume(&self) -> bool {
        self.live
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapHandle {
    pub gen: u64,
    pub nonce: u64,
}
pub struct SovereignGate {
    slots: Vec<CapSlot>,
    gen_counter: u64,
}

impl SovereignGate {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            gen_counter: 0,
        }
    }

    /// Lëshon një capability të re. Kthen handle (numra) për thirrësin.
    pub fn issue(&mut self, module_seal: u64) -> CapHandle {
        let gen = self.gen_counter;
        self.gen_counter += 1;
        self.slots.push(CapSlot::new(module_seal));
        CapHandle {
            gen,
            nonce: module_seal,
        }
    }

    /// Validon dhe konsumon. Një herë.
    /// Klon i handle-it → CAS(false→false) → AlreadyConsumed.
    pub fn validate(&self, handle: CapHandle) -> Result<LgcToken, SovErr> {
        let idx = usize::try_from(handle.gen).map_err(|_| SovErr::NotFound)?;
        let slot = self.slots.get(idx).ok_or(SovErr::NotFound)?;

        // Nonce-i është pjesë e capability contract. Kontrollohet PARA CAS-it:
        // një handle i manipuluar refuzohet pa djegur capability-n legjitime.
        match slot.expected_nonce == handle.nonce {
            true => {}
            false => {
                return Err(SovErr::NonceMismatch {
                    expected: slot.expected_nonce,
                    actual: handle.nonce,
                })
            }
        }

        // CAS atomik — vetëm hera e parë kalon.
        match slot.consume() {
            true => Ok(LgcToken::new()),
            false => Err(SovErr::AlreadyConsumed),
        }
    }
}
