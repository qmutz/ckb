use crate::{
    core::{BlockNumber, Capacity, EpochNumber},
    packed,
    prelude::*,
    U256,
};
use ckb_error::Error;
use ckb_rational::RationalU256;
use std::cmp::Ordering;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct BlockExt {
    pub received_at: u64,
    pub total_difficulty: U256,
    pub total_uncles_count: u64,
    pub verified: Option<bool>,
    pub txs_fees: Vec<Capacity>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TransactionInfo {
    // Block hash
    pub block_hash: packed::Byte32,
    pub block_number: BlockNumber,
    pub block_epoch: EpochNumberWithFraction,
    // Index in the block
    pub index: usize,
}

impl TransactionInfo {
    pub fn key(&self) -> packed::TransactionKey {
        packed::TransactionKey::new_builder()
            .block_hash(self.block_hash.clone())
            .index(self.index.pack())
            .build()
    }

    pub fn new(
        block_number: BlockNumber,
        block_epoch: EpochNumberWithFraction,
        block_hash: packed::Byte32,
        index: usize,
    ) -> Self {
        TransactionInfo {
            block_number,
            block_epoch,
            block_hash,
            index,
        }
    }

    pub fn is_cellbase(&self) -> bool {
        self.index == 0
    }

    pub fn is_genesis(&self) -> bool {
        self.block_number == 0
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct EpochExt {
    pub(crate) number: EpochNumber,
    pub(crate) base_block_reward: Capacity,
    pub(crate) remainder_reward: Capacity,
    pub(crate) previous_epoch_hash_rate: U256,
    pub(crate) last_block_hash_in_previous_epoch: packed::Byte32,
    pub(crate) start_number: BlockNumber,
    pub(crate) length: BlockNumber,
    pub(crate) compact_target: u32,
}

#[derive(Clone, Debug)]
pub struct EpochExtBuilder(pub(crate) EpochExt);

impl EpochExt {
    //
    // Simple Getters
    //

    pub fn number(&self) -> EpochNumber {
        self.number
    }

    pub fn base_block_reward(&self) -> &Capacity {
        &self.base_block_reward
    }

    pub fn remainder_reward(&self) -> &Capacity {
        &self.remainder_reward
    }

    pub fn previous_epoch_hash_rate(&self) -> &U256 {
        &self.previous_epoch_hash_rate
    }

    pub fn last_block_hash_in_previous_epoch(&self) -> packed::Byte32 {
        self.last_block_hash_in_previous_epoch.clone()
    }

    pub fn start_number(&self) -> BlockNumber {
        self.start_number
    }

    pub fn length(&self) -> BlockNumber {
        self.length
    }

    pub fn compact_target(&self) -> u32 {
        self.compact_target
    }

    //
    // Simple Setters
    //

    pub fn set_number(&mut self, number: BlockNumber) {
        self.number = number;
    }

    pub fn set_base_block_reward(&mut self, base_block_reward: Capacity) {
        self.base_block_reward = base_block_reward;
    }

    pub fn set_remainder_reward(&mut self, remainder_reward: Capacity) {
        self.remainder_reward = remainder_reward;
    }

    pub fn set_previous_epoch_hash_rate(&mut self, previous_epoch_hash_rate: U256) {
        self.previous_epoch_hash_rate = previous_epoch_hash_rate;
    }

    pub fn set_last_block_hash_in_previous_epoch(
        &mut self,
        last_block_hash_in_previous_epoch: packed::Byte32,
    ) {
        self.last_block_hash_in_previous_epoch = last_block_hash_in_previous_epoch;
    }

    pub fn set_start_number(&mut self, start_number: BlockNumber) {
        self.start_number = start_number;
    }

    pub fn set_length(&mut self, length: BlockNumber) {
        self.length = length;
    }

    pub fn set_compact_target(&mut self, compact_target: u32) {
        self.compact_target = compact_target;
    }

    //
    // Normal Methods
    //

    pub fn new_builder() -> EpochExtBuilder {
        EpochExtBuilder(EpochExt::default())
    }

    pub fn into_builder(self) -> EpochExtBuilder {
        EpochExtBuilder(self)
    }

    pub fn is_genesis(&self) -> bool {
        0 == self.number
    }

    pub fn block_reward(&self, number: BlockNumber) -> Result<Capacity, Error> {
        if number >= self.start_number()
            && number < self.start_number() + self.remainder_reward.as_u64()
        {
            self.base_block_reward
                .safe_add(Capacity::one())
                .map_err(Into::into)
        } else {
            Ok(self.base_block_reward)
        }
    }

    pub fn number_with_fraction(&self, number: BlockNumber) -> EpochNumberWithFraction {
        debug_assert!(
            number >= self.start_number() && number < self.start_number() + self.length()
        );
        EpochNumberWithFraction::new(self.number(), number - self.start_number(), self.length())
    }

    // We name this issuance since it covers multiple parts: block reward,
    // NervosDAO issuance as well as treasury part.
    pub fn secondary_block_issuance(
        &self,
        block_number: BlockNumber,
        secondary_epoch_issuance: Capacity,
    ) -> Result<Capacity, Error> {
        let mut g2 = Capacity::shannons(secondary_epoch_issuance.as_u64() / self.length());
        let remainder = secondary_epoch_issuance.as_u64() % self.length();
        if block_number >= self.start_number() && block_number < self.start_number() + remainder {
            g2 = g2.safe_add(Capacity::one())?;
        }
        Ok(g2)
    }
}

impl EpochExtBuilder {
    //
    // Simple Setters
    //

    pub fn number(mut self, number: BlockNumber) -> Self {
        self.0.set_number(number);
        self
    }

    pub fn base_block_reward(mut self, base_block_reward: Capacity) -> Self {
        self.0.set_base_block_reward(base_block_reward);
        self
    }

    pub fn remainder_reward(mut self, remainder_reward: Capacity) -> Self {
        self.0.set_remainder_reward(remainder_reward);
        self
    }

    pub fn previous_epoch_hash_rate(mut self, previous_epoch_hash_rate: U256) -> Self {
        self.0
            .set_previous_epoch_hash_rate(previous_epoch_hash_rate);
        self
    }

    pub fn last_block_hash_in_previous_epoch(
        mut self,
        last_block_hash_in_previous_epoch: packed::Byte32,
    ) -> Self {
        self.0
            .set_last_block_hash_in_previous_epoch(last_block_hash_in_previous_epoch);
        self
    }

    pub fn start_number(mut self, start_number: BlockNumber) -> Self {
        self.0.set_start_number(start_number);
        self
    }

    pub fn length(mut self, length: BlockNumber) -> Self {
        self.0.set_length(length);
        self
    }

    pub fn compact_target(mut self, compact_target: u32) -> Self {
        self.0.set_compact_target(compact_target);
        self
    }

    //
    // Normal Methods
    //
    // The `new` methods are unnecessary. Creating `EpochExtBuilder` from `EpochExt`, it's enough.

    pub fn build(self) -> EpochExt {
        self.0
    }
}

/// Represents an epoch number with a fraction unit, it can be
/// used to accurately represent the position for a block within
/// an epoch.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct EpochNumberWithFraction(u64);

impl fmt::Display for EpochNumberWithFraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Epoch {{ number: {}, index: {}, length: {} }}",
            self.number(),
            self.index(),
            self.length()
        )
    }
}

impl FromStr for EpochNumberWithFraction {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = u64::from_str(s)?;
        Ok(EpochNumberWithFraction(v))
    }
}

impl PartialOrd for EpochNumberWithFraction {
    fn partial_cmp(&self, other: &EpochNumberWithFraction) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EpochNumberWithFraction {
    fn cmp(&self, other: &EpochNumberWithFraction) -> Ordering {
        if self.number() < other.number() {
            Ordering::Less
        } else if self.number() > other.number() {
            Ordering::Greater
        } else {
            let a = self.index() * other.length();
            let b = other.index() * self.length();
            a.cmp(&b)
        }
    }
}

impl EpochNumberWithFraction {
    pub const NUMBER_OFFSET: usize = 0;
    pub const NUMBER_BITS: usize = 24;
    pub const NUMBER_MAXIMUM_VALUE: u64 = (1u64 << Self::NUMBER_BITS);
    pub const NUMBER_MASK: u64 = (Self::NUMBER_MAXIMUM_VALUE - 1);
    pub const INDEX_OFFSET: usize = Self::NUMBER_BITS;
    pub const INDEX_BITS: usize = 16;
    pub const INDEX_MAXIMUM_VALUE: u64 = (1u64 << Self::INDEX_BITS);
    pub const INDEX_MASK: u64 = (Self::INDEX_MAXIMUM_VALUE - 1);
    pub const LENGTH_OFFSET: usize = Self::NUMBER_BITS + Self::INDEX_BITS;
    pub const LENGTH_BITS: usize = 16;
    pub const LENGTH_MAXIMUM_VALUE: u64 = (1u64 << Self::LENGTH_BITS);
    pub const LENGTH_MASK: u64 = (Self::LENGTH_MAXIMUM_VALUE - 1);

    pub fn new(number: u64, index: u64, length: u64) -> EpochNumberWithFraction {
        debug_assert!(number < Self::NUMBER_MAXIMUM_VALUE);
        debug_assert!(index < Self::INDEX_MAXIMUM_VALUE);
        debug_assert!(length < Self::LENGTH_MAXIMUM_VALUE);
        debug_assert!(length > 0);
        Self::new_unchecked(number, index, length)
    }

    pub const fn new_unchecked(number: u64, index: u64, length: u64) -> Self {
        EpochNumberWithFraction(
            (length << Self::LENGTH_OFFSET)
                | (index << Self::INDEX_OFFSET)
                | (number << Self::NUMBER_OFFSET),
        )
    }

    pub fn number(self) -> EpochNumber {
        (self.0 >> Self::NUMBER_OFFSET) & Self::NUMBER_MASK
    }

    pub fn index(self) -> u64 {
        (self.0 >> Self::INDEX_OFFSET) & Self::INDEX_MASK
    }

    pub fn length(self) -> u64 {
        (self.0 >> Self::LENGTH_OFFSET) & Self::LENGTH_MASK
    }

    pub fn full_value(self) -> u64 {
        self.0
    }

    // One caveat here, is that if the user specifies a zero epoch length either
    // delibrately, or by accident, calling to_rational() after that might
    // result in a division by zero panic. To prevent that, this method would
    // automatically rewrite the value to epoch index 0 with epoch length to
    // prevent panics
    pub fn from_full_value(value: u64) -> Self {
        let epoch = Self(value);
        if epoch.length() == 0 {
            Self::new(epoch.number(), 0, 1)
        } else {
            epoch
        }
    }

    pub fn to_rational(self) -> RationalU256 {
        RationalU256::new(self.index().into(), self.length().into()) + U256::from(self.number())
    }
}
