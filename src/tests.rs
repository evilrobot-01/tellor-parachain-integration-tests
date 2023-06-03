use super::*;
use frame_support::assert_ok;
use parachains::{
    evm::contracts::REGISTRY_CONTRACT_ADDRESS, evm::contracts::REGISTRY_CONTRACT_BYTECODE,
    evm::ALITH, evm::PALLET_DERIVATIVE_ACCOUNT,
};
use sp_runtime::app_crypto::sp_core::{H160, U256};
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
    deploy_contracts();

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
        let parachain_registered = parachains::evm::contracts::parachain_registered(3_000);
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

fn deploy_contracts() {
    use moonbase_runtime::{RuntimeEvent, RuntimeOrigin, System, EVM};

    let gas_limit = 10_000_000;
    let max_fee_per_gas = U256::from(1_250_000_000);

    EvmParachain::execute_with(|| {
        // create parachain registry contract
        assert_ok!(EVM::create(
            RuntimeOrigin::root(),
            ALITH.into(),
            REGISTRY_CONTRACT_BYTECODE.into(),
            U256::zero(),
            gas_limit,
            max_fee_per_gas,
            None,
            None,
            Vec::new()
        ));
        System::assert_last_event(RuntimeEvent::EVM(pallet_evm::Event::Created {
            address: REGISTRY_CONTRACT_ADDRESS.into(),
        }));
        System::reset_events()
    });
}
