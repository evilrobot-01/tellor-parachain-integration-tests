use super::*;
use crate::{
    parachains::evm::contracts::governance::GOVERNANCE_CONTRACT_ADDRESS,
    parachains::evm::contracts::staking::STAKING_CONTRACT_ADDRESS,
    parachains::evm::{BALTHAZAR, XCTRB_ADDRESS},
    relay_chain::BOB,
};
use frame_support::assert_ok;
use frame_support::traits::UnixTime;
use oracle_consumer_runtime::Timestamp;
use parachains::{
    evm::contracts::registry::REGISTRY_CONTRACT_ADDRESS, evm::PALLET_DERIVATIVE_ACCOUNT,
};
use sp_runtime::app_crypto::sp_core::H160;
use sp_runtime::app_crypto::ByteArray;
use sp_runtime::traits::{Hash, Keccak256};
use sp_runtime::BoundedVec;
use std::sync::Once;
use xcm_emulator::TestExt;

static INIT: Once = Once::new();
fn init_tracing() {
    INIT.call_once(|| {
        // Add test tracing (from sp_tracing::init_for_tests()) but filtering for xcm logs only
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_env_filter("xcm=trace,system::events=trace,evm=trace") // Comment out this line to see all traces
            .with_test_writer()
            .init();
    });
}

#[test]
fn registers() {
    init_tracing();
    Network::reset();

    // deploy parachain registry contract to evm parachain
    EvmParachain::execute_with(|| {
        parachains::evm::contracts::registry::deploy();
    });

    // register oracle consumer parachain with contracts on evm parachain via xcm
    OracleConsumerParachain::execute_with(|| {
        parachains::oracle_consumer::register(2_000);
    });

    // ensure events emitted on evm parachain
    EvmParachain::execute_with(|| {
        use moonbase_runtime::{RuntimeEvent, System};
        use pallet_evm::{ExitReason::Succeed, ExitSucceed::Stopped};

        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::Ethereum(pallet_ethereum::Event::Executed {
                from: H160(PALLET_DERIVATIVE_ACCOUNT), // parachain registry contract called via pallet derivative account on evm parachain
                to: H160(REGISTRY_CONTRACT_ADDRESS),
                transaction_hash: _,
                exit_reason: Succeed(Stopped)
            })
        )));
        // ensure ParachainRegistered event emitted by parachain registry contract
        let (topics, data) = parachains::evm::contracts::registry::parachain_registered(3_000);
        System::assert_has_event(
            pallet_evm::Event::Log {
                log: ethereum::Log {
                    address: REGISTRY_CONTRACT_ADDRESS.into(),
                    topics,
                    data,
                },
            }
            .into(),
        );
    });
}

#[test]
fn deploys_contracts() {
    init_tracing();
    Network::reset();

    // deploy and init contracts
    EvmParachain::execute_with(|| {
        use parachains::{evm::contracts::*, evm::ALITH};
        // deploy contracts
        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
        // init contracts with addresses
        staking::init(&GOVERNANCE_CONTRACT_ADDRESS);
        governance::init(&STAKING_CONTRACT_ADDRESS);
    });
}

#[test]
fn creates_xctrb_asset() {
    init_tracing();
    Network::reset();

    // register TRB as foreign asset
    EvmParachain::execute_with(|| parachains::evm::create_xctrb_asset());
}

#[test]
fn stakes() {
    init_tracing();
    Network::reset();

    // create trb asset and deploy contracts
    EvmParachain::execute_with(|| {
        use parachains::{evm::contracts::*, evm::ALITH};

        parachains::evm::create_xctrb_asset();

        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
        staking::init(&GOVERNANCE_CONTRACT_ADDRESS);
    });

    // register oracle consumer parachain with contracts on evm parachain via xcm
    OracleConsumerParachain::execute_with(|| {
        parachains::oracle_consumer::register(2_000);
    });

    // mint, approve and stake trb for oracle consumer parachain
    let amount = <oracle_consumer_runtime::Runtime as tellor::Config>::MinimumStakeAmount::get();
    EvmParachain::execute_with(|| {
        use parachains::evm::contracts::staking;
        let asset = u128::from_be_bytes(XCTRB_ADDRESS[4..].try_into().unwrap());

        staking::mint(asset, &BALTHAZAR, amount);
        staking::approve(&BALTHAZAR, asset, &STAKING_CONTRACT_ADDRESS, amount);
        staking::stake(&BALTHAZAR, 3_000, BOB.to_raw_vec(), amount);
    });

    // ensure stake reported to pallet on oracle consumer parachain
    OracleConsumerParachain::execute_with(|| {
        use oracle_consumer_runtime::System;
        System::assert_has_event(
            tellor::Event::NewStakerReported {
                staker: BOB,
                amount: amount.into(),
                address: H160(BALTHAZAR),
            }
            .into(),
        );
    });
}

#[test]
fn submits_value() {
    init_tracing();
    Network::reset();

    // create trb asset and deploy contracts
    EvmParachain::execute_with(|| {
        use parachains::{evm::contracts::*, evm::ALITH};

        parachains::evm::create_xctrb_asset();

        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
        staking::init(&GOVERNANCE_CONTRACT_ADDRESS);
    });

    // register oracle consumer parachain with contracts on evm parachain via xcm
    OracleConsumerParachain::execute_with(|| {
        parachains::oracle_consumer::register(2_000);
    });

    // mint, approve and stake trb for oracle consumer parachain
    let amount = <oracle_consumer_runtime::Runtime as tellor::Config>::MinimumStakeAmount::get();
    EvmParachain::execute_with(|| {
        use parachains::evm::contracts::staking;
        let asset = u128::from_be_bytes(XCTRB_ADDRESS[4..].try_into().unwrap());

        staking::mint(asset, &BALTHAZAR, amount);
        staking::approve(&BALTHAZAR, asset, &STAKING_CONTRACT_ADDRESS, amount);
        staking::stake(&BALTHAZAR, 3_000, BOB.to_raw_vec(), amount);
    });

    // submit value to oracle
    OracleConsumerParachain::execute_with(|| {
        use oracle_consumer_runtime::{Runtime, RuntimeOrigin, System, Tellor};

        type QueryData = BoundedVec<u8, <Runtime as tellor::Config>::MaxQueryDataLength>;
        type Value = BoundedVec<u8, <Runtime as tellor::Config>::MaxValueLength>;

        let query_data: QueryData = b"hello tellor".to_vec().try_into().unwrap();
        let query_id = Keccak256::hash(query_data.as_slice());
        let value: Value = b"hey!".to_vec().try_into().unwrap();
        let nonce = 0;
        let timestamp = <Timestamp as UnixTime>::now().as_secs();

        assert_ok!(Tellor::submit_value(
            RuntimeOrigin::signed(BOB),
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
    });
}
