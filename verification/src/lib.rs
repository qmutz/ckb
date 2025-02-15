#[macro_use]
extern crate enum_display_derive;

mod block_verifier;
mod contextual_block_verifier;
mod convert;
mod error;
mod header_verifier;
mod transaction_verifier;
pub mod txs_verify_cache;
mod uncles_verifier;

#[cfg(test)]
mod tests;

pub use crate::block_verifier::{BlockVerifier, HeaderResolverWrapper};
pub use crate::contextual_block_verifier::{ContextualBlockVerifier, Switch, VerifyContext};
pub use crate::error::{
    BlockError, BlockErrorKind, BlockTransactionsError, CellbaseError, CommitError, EpochError,
    HeaderError, HeaderErrorKind, InvalidParentError, NumberError, PowError, TimestampError,
    TransactionError, UnclesError, UnknownParentError,
};
pub use crate::header_verifier::{HeaderResolver, HeaderVerifier};
pub use crate::transaction_verifier::{
    ContextualTransactionVerifier, ScriptVerifier, TransactionVerifier,
};

pub const ALLOWED_FUTURE_BLOCKTIME: u64 = 15 * 1000; // 15 Second

pub(crate) const LOG_TARGET: &str = "ckb-chain";

pub trait Verifier {
    type Target;
    fn verify(&self, target: &Self::Target) -> Result<(), ckb_error::Error>;
}
