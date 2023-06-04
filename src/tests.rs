use super::*;
use crate::parachains::evm::contracts::governance::GOVERNANCE_CONTRACT_ADDRESS;
use crate::parachains::evm::contracts::staking::STAKING_CONTRACT_ADDRESS;
use crate::parachains::evm::{BALTHAZAR, XCTRB_ADDRESS};
use crate::relay_chain::BOB;
use frame_support::assert_ok;
use parachains::{
    evm::contracts::registry::REGISTRY_CONTRACT_ADDRESS, evm::PALLET_DERIVATIVE_ACCOUNT,
};
use sp_runtime::app_crypto::sp_core::H160;
use sp_runtime::app_crypto::ByteArray;
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
        use parachains::evm::contracts::registry;

        registry::deploy();
    });

    // register oracle consumer parachain with contracts on evm parachain via xcm
    OracleConsumerParachain::execute_with(|| {
        use oracle_consumer_runtime::{RuntimeEvent, RuntimeOrigin, System, Tellor};
        assert_ok!(Tellor::register(RuntimeOrigin::root()));
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::Tellor(tellor::Event::RegistrationSent {
                para_id: 2_000,
                contract_address: H160(REGISTRY_CONTRACT_ADDRESS),
                weights: _
            })
        )));
    });

    // ensure resulting evm parachain events
    EvmParachain::execute_with(|| {
        use moonbase_runtime::{RuntimeEvent, System};
        use pallet_evm::{ExitReason::Succeed, ExitSucceed::Stopped};
        // parachain registry contract called via pallet derivative account
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::Ethereum(pallet_ethereum::Event::Executed {
                from: H160(PALLET_DERIVATIVE_ACCOUNT),
                to: H160(REGISTRY_CONTRACT_ADDRESS),
                transaction_hash: _,
                exit_reason: Succeed(Stopped)
            })
        )));
        // parachain registered event emitted by parachain registry contract
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
        use parachains::{
            evm::contracts::governance, evm::contracts::registry, evm::contracts::staking,
            evm::ALITH,
        };
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
fn creates_xctrb() {
    init_tracing();

    Network::reset();

    // register TRB as foreign asset
    EvmParachain::execute_with(|| {
        parachains::evm::create_xctrb();
    });
}

#[test]
fn stakes() {
    init_tracing();

    Network::reset();

    // create trb asset and deploy contracts
    EvmParachain::execute_with(|| {
        use parachains::{
            evm::contracts::governance, evm::contracts::registry, evm::contracts::staking,
            evm::ALITH,
        };

        parachains::evm::create_xctrb();

        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
        staking::init(&GOVERNANCE_CONTRACT_ADDRESS);
    });

    // register oracle consumer parachain with contracts on evm parachain via xcm
    OracleConsumerParachain::execute_with(|| {
        use oracle_consumer_runtime::{RuntimeOrigin, Tellor};
        assert_ok!(Tellor::register(RuntimeOrigin::root()));
    });

    // approve and stake trb for oracle consumer parachain
    let amount = 100 * 10u128.saturating_pow(18);
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
