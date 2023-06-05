use super::*;
use ethabi::{encode, Event, EventParam, Hash, ParamType, Token};
use frame_support::assert_ok;
use sp_runtime::app_crypto::sp_core::U256;

pub(crate) mod governance;
pub(crate) mod registry;
pub(crate) mod staking;

const GAS_LIMIT: u64 = 10_000_000;
const MAX_FEE_PER_GAS: u128 = 1_250_000_000;
