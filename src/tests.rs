use super::*;
use crate::parachains::evm::contracts::governance::GOVERNANCE_CONTRACT_ADDRESS;
use crate::parachains::evm::contracts::staking::STAKING_CONTRACT_ADDRESS;
use crate::parachains::evm::XCTRB_ADDRESS;
use frame_support::assert_ok;
use parachains::{
    evm::contracts::registry::REGISTRY_CONTRACT_ADDRESS, evm::PALLET_DERIVATIVE_ACCOUNT,
};
use sp_runtime::app_crypto::sp_core::H160;
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
        use moonbase_runtime::System;
        use parachains::evm::contracts::registry;
        registry::deploy();
        System::reset_events()
    });

    // register pallet on oracle consumer parachain with contracts on evm parachain via xcm
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
        let parachain_registered =
            parachains::evm::contracts::registry::parachain_registered(3_000);
        assert!(System::events().iter().any(|r| {
            match &r.event {
                RuntimeEvent::EVM(pallet_evm::Event::Log { log }) => {
                    log.address.0 == REGISTRY_CONTRACT_ADDRESS
                        && log.topics == parachain_registered.0
                        && log.data == parachain_registered.1
                }
                _ => false,
            }
        }));
    });
}

#[test]
fn deploys_contracts() {
    init_tracing();

    Network::reset();

    EvmParachain::execute_with(|| {
        use parachains::{
            evm::contracts::governance, evm::contracts::registry, evm::contracts::staking,
            evm::ALITH,
        };
        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
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
