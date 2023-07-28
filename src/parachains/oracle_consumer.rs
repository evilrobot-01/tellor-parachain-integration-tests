use super::*;
use crate::relay_chain::*;
use core::time::Duration;
use ethabi::ethereum_types::H256;
use frame_support::{
    assert_ok,
    traits::{fungible::Inspect, UnixTime},
    BoundedVec,
};
use oracle_consumer_runtime::{
    Balance, Balances, Runtime, RuntimeOrigin, System, Tellor, Timestamp,
};
use sp_runtime::{
    traits::{AccountIdConversion, Hash, Keccak256},
    AccountId32,
};

pub(crate) fn new_ext(para_id: u32) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    // set parachain id
    let parachain_info_config = parachain_info::GenesisConfig {
        parachain_id: para_id.into(),
    };
    <parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
        &parachain_info_config,
        &mut t,
    )
    .unwrap();

    // set initial balances
    let pallet_id = <Runtime as tellor::Config>::PalletId::get();
    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (BOB, Balances::minimum_balance()), // required to claim tips
            (CHARLIE, 10 * 10u128.pow(12)),     // required for tips
            (DAVE, 55 * 10u128.pow(12)),        // required for disputes
            (pallet_id.into_account_truncating(), 1 * 10u128.pow(12)), // required for xcm fees
            // initialise sub-accounts
            (
                pallet_id.into_sub_account_truncating(b"tips"),
                Balances::minimum_balance(),
            ),
            (
                pallet_id.into_sub_account_truncating(b"staking"),
                Balances::minimum_balance(),
            ),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        pallet_timestamp::Now::<Runtime>::put(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Current time is always after unix epoch; qed")
                .as_millis() as u64,
        );
    });
    ext
}

pub(crate) fn feed_id(
    query_id: H256,
    reward: Balance,
    start_time: u64,
    interval: u64,
    window: u64,
    price_threshold: u16,
    reward_increase_per_second: Balance,
) -> H256 {
    use ethabi::Token::*;
    Keccak256::hash(&ethabi::encode(&vec![
        FixedBytes(query_id.0.to_vec()),
        Uint(reward.into()),
        Uint(start_time.into()),
        Uint(interval.into()),
        Uint(window.into()),
        Uint(price_threshold.into()),
        Uint(reward_increase_per_second.into()),
    ]))
}

pub(crate) fn register(evm_para_id: u32) {
    use tellor::{weights::WeightInfo, MAX_VOTE_ROUNDS};
    assert_ok!(Tellor::register(RuntimeOrigin::root()));
    let weights = tellor::Weights {
        report_stake_deposited: <() as WeightInfo>::report_stake_deposited().ref_time(),
        report_staking_withdraw_request: <() as WeightInfo>::report_staking_withdraw_request()
            .ref_time(),
        report_stake_withdrawn: <() as WeightInfo>::report_stake_withdrawn().ref_time(),
        report_vote_tallied: <() as WeightInfo>::report_vote_tallied().ref_time(),
        report_vote_executed: <() as WeightInfo>::report_vote_executed(MAX_VOTE_ROUNDS.into())
            .ref_time(),
        report_slash: <() as WeightInfo>::report_slash().ref_time(),
    };
    System::assert_has_event(
        tellor::Event::RegistrationSent {
            para_id: evm_para_id,
            contract_address: evm::contracts::registry::REGISTRY_CONTRACT_ADDRESS.into(),
            weights,
        }
        .into(),
    );
}

pub(crate) fn submit_value(
    reporter: AccountId32,
    query_data: &[u8],
    value: &[u8],
) -> (tellor::QueryId, tellor::Timestamp) {
    type QueryData = BoundedVec<u8, <Runtime as tellor::Config>::MaxQueryDataLength>;
    type Value = BoundedVec<u8, <Runtime as tellor::Config>::MaxValueLength>;

    let query_data: QueryData = query_data.to_vec().try_into().unwrap();
    let query_id = Keccak256::hash(query_data.as_slice());
    let value: Value = value.to_vec().try_into().unwrap();
    let nonce = 0;
    let timestamp = <Timestamp as UnixTime>::now().as_secs();

    assert_ok!(Tellor::submit_value(
        RuntimeOrigin::signed(reporter),
        query_id,
        value.clone(),
        nonce,
        query_data.clone()
    ));
    System::assert_has_event(
        tellor::Event::NewReport {
            query_id,
            time: timestamp,
            value,
            nonce,
            query_data,
            reporter: BOB,
        }
        .into(),
    );
    (query_id, timestamp)
}

pub(crate) fn advance_time(time_in_secs: u64) {
    let now = <Timestamp as UnixTime>::now();
    pallet_timestamp::Now::<Runtime>::set(
        (now + Duration::from_secs(time_in_secs)).as_millis() as u64
    );
}
