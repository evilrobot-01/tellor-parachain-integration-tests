use super::*;
use ethabi::{encode, Event, EventParam, ParamType, Token};
use frame_support::assert_ok;
use pallet_evm::{ExitReason::Succeed, ExitSucceed::Stopped};
use sp_runtime::app_crypto::sp_core::U256;

pub(crate) mod governance;
pub(crate) mod registry;
pub(crate) mod staking;

const GAS_LIMIT: u64 = 10_000_000;
const MAX_FEE_PER_GAS: u128 = 125_000_000_000;
