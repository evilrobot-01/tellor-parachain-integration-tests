use super::*;
use crate::relay_chain::*;
use frame_support::traits::UnixTime;
use frame_support::{assert_ok, BoundedVec};
use oracle_consumer_runtime::{Runtime, RuntimeOrigin, System, Tellor, Timestamp};
use sp_runtime::traits::{Hash, Keccak256};
use sp_runtime::AccountId32;

const INITIAL_BALANCE: u128 = 1_000 * 10u128.pow(12);
const PALLET_ACCOUNT: [u8; 32] = [
    109, 111, 100, 108, 112, 121, 47, 116, 101, 108, 108, 114, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

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
    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (DAVE, INITIAL_BALANCE), // required for disputes
            (PALLET_ACCOUNT.into(), INITIAL_BALANCE),
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

pub(crate) fn register(evm_para_id: u32) {
    use tellor::weights::WeightInfo;
    assert_ok!(Tellor::register(RuntimeOrigin::root()));
    let weights = tellor::Weights {
        report_stake_deposited: <() as WeightInfo>::report_stake_deposited().ref_time(),
        report_staking_withdraw_request: <() as WeightInfo>::report_staking_withdraw_request()
            .ref_time(),
        report_stake_withdrawn: <() as WeightInfo>::report_stake_withdrawn().ref_time(),
        report_vote_tallied: <() as WeightInfo>::report_vote_tallied().ref_time(),
        report_vote_executed: <() as WeightInfo>::report_vote_executed(u8::MAX.into()).ref_time(),
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
