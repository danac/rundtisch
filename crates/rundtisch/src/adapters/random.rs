use crate::traits::random::{RandomError, RandomSource};
#[cfg(test)]
use std::sync::Mutex;

/// `getrandom::fill` — default on native.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), RandomError> {
        getrandom::fill(dest).map_err(|err| RandomError::Backend(err.to_string()))
    }
}

/// `crypto.getRandomValues` via `js_sys` — default on `wasm32` with `d1`.
#[cfg(all(target_arch = "wasm32", feature = "d1"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct WorkerRandom;

#[cfg(all(target_arch = "wasm32", feature = "d1"))]
impl RandomSource for WorkerRandom {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), RandomError> {
        if dest.is_empty() {
            return Ok(());
        }
        use js_sys::{Reflect, Uint8Array};
        use wasm_bindgen::JsCast;

        let global = js_sys::global();
        let crypto = Reflect::get(&global, &"crypto".into())
            .map_err(|err| RandomError::Backend(format!("{err:?}")))?;
        if crypto.is_undefined() || crypto.is_null() {
            return Err(RandomError::Unavailable);
        }
        let func = Reflect::get(&crypto, &"getRandomValues".into())
            .map_err(|err| RandomError::Backend(format!("{err:?}")))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| RandomError::Unavailable)?;
        let array = Uint8Array::new_with_length(dest.len() as u32);
        func.call1(&crypto, array.as_ref())
            .map_err(|err| RandomError::Backend(format!("{err:?}")))?;
        array.copy_to(dest);
        Ok(())
    }
}

#[cfg(all(target_arch = "wasm32", feature = "d1"))]
pub use WorkerRandom as DefaultRandom;

#[cfg(not(all(target_arch = "wasm32", feature = "d1")))]
pub use OsRandom as DefaultRandom;

/// Cycles a fixed byte string. Tests override [`crate::traits::Platform::random`].
#[cfg(test)]
pub struct ReplayRandom {
    bytes: Vec<u8>,
    offset: Mutex<usize>,
}

#[cfg(test)]
impl ReplayRandom {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
            offset: Mutex::new(0),
        }
    }
}

#[cfg(test)]
impl RandomSource for ReplayRandom {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), RandomError> {
        if self.bytes.is_empty() {
            return Err(RandomError::Unavailable);
        }
        let mut offset = self
            .offset
            .lock()
            .map_err(|_| RandomError::Backend("replay lock poisoned".into()))?;
        for slot in dest.iter_mut() {
            *slot = self.bytes[*offset % self.bytes.len()];
            *offset += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_random_fills() {
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        OsRandom.fill_bytes(&mut a).unwrap();
        OsRandom.fill_bytes(&mut b).unwrap();
        assert_ne!(a, [0u8; 16]);
        assert_ne!(a, b);
    }

    #[test]
    fn replay_random_is_deterministic() {
        let rng = ReplayRandom::new(vec![1, 2, 3]);
        let mut dest = [0u8; 5];
        rng.fill_bytes(&mut dest).unwrap();
        assert_eq!(dest, [1, 2, 3, 1, 2]);
    }
}
