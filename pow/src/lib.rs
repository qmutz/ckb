use byteorder::{ByteOrder, LittleEndian};
use ckb_types::{
    packed::{Byte32, Header},
    prelude::*,
};
use serde_derive::{Deserialize, Serialize};
use std::any::Any;
use std::fmt;
use std::sync::Arc;

mod dummy;
mod eaglesong;

pub use crate::dummy::DummyPowEngine;
pub use crate::eaglesong::EaglesongPowEngine;

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
#[serde(tag = "func", content = "params")]
pub enum Pow {
    Dummy,
    Eaglesong,
}

impl fmt::Display for Pow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pow::Dummy => write!(f, "Dummy"),
            Pow::Eaglesong => write!(f, "Eaglesong"),
        }
    }
}

impl Pow {
    pub fn engine(&self) -> Arc<dyn PowEngine> {
        match *self {
            Pow::Dummy => Arc::new(DummyPowEngine),
            Pow::Eaglesong => Arc::new(EaglesongPowEngine),
        }
    }
}

pub fn pow_message(pow_hash: &Byte32, nonce: u64) -> [u8; 40] {
    let mut message = [0; 40];
    message[8..40].copy_from_slice(pow_hash.as_slice());
    LittleEndian::write_u64(&mut message, nonce);
    message
}

pub trait PowEngine: Send + Sync + AsAny {
    fn verify(&self, header: &Header) -> bool;
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ckb_hash::blake2b_256;
    #[test]
    fn test_pow_message() {
        let zero_hash = blake2b_256(&[]).pack();
        let nonce = u64::max_value();
        let message = pow_message(&zero_hash, nonce);
        assert_eq!(
            message.to_vec(),
            [
                255, 255, 255, 255, 255, 255, 255, 255, 68, 244, 198, 151, 68, 213, 248, 197, 93,
                100, 32, 98, 148, 157, 202, 228, 155, 196, 231, 239, 67, 211, 136, 197, 161, 47,
                66, 181, 99, 61, 22, 62
            ]
            .to_vec()
        );
    }
}
