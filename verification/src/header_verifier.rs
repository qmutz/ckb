use super::Verifier;
use crate::{
    BlockErrorKind, NumberError, PowError, TimestampError, UnknownParentError,
    ALLOWED_FUTURE_BLOCKTIME,
};
use ckb_error::Error;
use ckb_pow::PowEngine;
use ckb_traits::BlockMedianTimeContext;
use ckb_types::{constants::HEADER_VERSION, core::HeaderView};
use faketime::unix_time_as_millis;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait HeaderResolver {
    fn header(&self) -> &HeaderView;
    /// resolves parent header
    fn parent(&self) -> Option<&HeaderView>;
}

pub struct HeaderVerifier<'a, T, M> {
    pub pow: Arc<dyn PowEngine>,
    block_median_time_context: &'a M,
    _phantom: PhantomData<T>,
}

impl<'a, T, M: BlockMedianTimeContext> HeaderVerifier<'a, T, M> {
    pub fn new(block_median_time_context: &'a M, pow: Arc<dyn PowEngine>) -> Self {
        HeaderVerifier {
            pow,
            block_median_time_context,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: HeaderResolver, M: BlockMedianTimeContext> Verifier for HeaderVerifier<'a, T, M> {
    type Target = T;
    fn verify(&self, target: &T) -> Result<(), Error> {
        let header = target.header();
        VersionVerifier::new(header).verify()?;
        // POW check first
        PowVerifier::new(header, &self.pow).verify()?;
        let parent = target.parent().ok_or_else(|| UnknownParentError {
            parent_hash: header.parent_hash().to_owned(),
        })?;
        NumberVerifier::new(parent, header).verify()?;
        TimestampVerifier::new(self.block_median_time_context, header).verify()?;
        Ok(())
    }
}

pub struct VersionVerifier<'a> {
    header: &'a HeaderView,
}

impl<'a> VersionVerifier<'a> {
    pub fn new(header: &'a HeaderView) -> Self {
        VersionVerifier { header }
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.header.version() != HEADER_VERSION {
            return Err(BlockErrorKind::Version.into());
        }
        Ok(())
    }
}

pub struct TimestampVerifier<'a, M> {
    header: &'a HeaderView,
    block_median_time_context: &'a M,
    now: u64,
}

impl<'a, M: BlockMedianTimeContext> TimestampVerifier<'a, M> {
    pub fn new(block_median_time_context: &'a M, header: &'a HeaderView) -> Self {
        TimestampVerifier {
            block_median_time_context,
            header,
            now: unix_time_as_millis(),
        }
    }

    pub fn verify(&self) -> Result<(), Error> {
        // skip genesis block
        if self.header.is_genesis() {
            return Ok(());
        }

        let min = self
            .block_median_time_context
            .block_median_time(&self.header.data().raw().parent_hash());
        if self.header.timestamp() <= min {
            return Err(TimestampError::BlockTimeTooOld {
                min,
                actual: self.header.timestamp(),
            }
            .into());
        }
        let max = self.now + ALLOWED_FUTURE_BLOCKTIME;
        if self.header.timestamp() > max {
            return Err(TimestampError::BlockTimeTooNew {
                max,
                actual: self.header.timestamp(),
            }
            .into());
        }
        Ok(())
    }
}

pub struct NumberVerifier<'a> {
    parent: &'a HeaderView,
    header: &'a HeaderView,
}

impl<'a> NumberVerifier<'a> {
    pub fn new(parent: &'a HeaderView, header: &'a HeaderView) -> Self {
        NumberVerifier { parent, header }
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.header.number() != self.parent.number() + 1 {
            return Err(NumberError {
                expected: self.parent.number() + 1,
                actual: self.header.number(),
            }
            .into());
        }
        Ok(())
    }
}

pub struct PowVerifier<'a> {
    header: &'a HeaderView,
    pow: Arc<dyn PowEngine>,
}

impl<'a> PowVerifier<'a> {
    pub fn new(header: &'a HeaderView, pow: &Arc<dyn PowEngine>) -> Self {
        PowVerifier {
            header,
            pow: Arc::clone(pow),
        }
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.pow.verify(&self.header.data()) {
            Ok(())
        } else {
            Err(PowError::InvalidNonce.into())
        }
    }
}
