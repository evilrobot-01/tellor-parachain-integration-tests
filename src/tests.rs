use super::*;
use frame_support::{assert_ok, traits::Get};
use parachains::evm::{
    contracts::governance::GOVERNANCE_CONTRACT_ADDRESS,
    contracts::registry::REGISTRY_CONTRACT_ADDRESS, contracts::staking::STAKING_CONTRACT_ADDRESS,
    BALTHAZAR, DOROTHY, PALLET_DERIVATIVE_ACCOUNT, XCTRB_ADDRESS,
};
use relay_chain::{BOB, DAVE};
use sp_runtime::{
    app_crypto::sp_core::H160,
    app_crypto::ByteArray,
    traits::{Hash, Keccak256},
};
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

    // register oracle consumer parachain with contracts on evm parachain via tellor pallet
    OracleConsumerParachain::execute_with(|| {
        parachains::oracle_consumer::register(2_000);
    });

    // ensure registry contract called on evm parachain and expected events emitted
    EvmParachain::execute_with(|| {
        use parachains::evm::contracts::registry;
        // ensure registry contract called (via pallet derivative account on evm parachain)
        registry::assert_executed(&PALLET_DERIVATIVE_ACCOUNT);
        // ensure ParachainRegistered event emitted by parachain registry contract
        registry::assert_parachain_registered_event(3_000);
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
        // create asset
        parachains::evm::create_xctrb_asset();
        // deploy contracts
        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
        // init contracts with addresses
        staking::init(&GOVERNANCE_CONTRACT_ADDRESS);
    });

    // register oracle consumer parachain with contracts on evm parachain via tellor pallet
    OracleConsumerParachain::execute_with(|| {
        parachains::oracle_consumer::register(2_000);
    });

    // mint, approve and stake trb in staking contract for oracle consumer parachain
    let amount = <oracle_consumer_runtime::Runtime as tellor::Config>::MinimumStakeAmount::get();
    EvmParachain::execute_with(|| {
        use parachains::evm::contracts::staking;
        let asset = u128::from_be_bytes(XCTRB_ADDRESS[4..].try_into().unwrap());
        staking::mint(asset, &BALTHAZAR, amount);
        staking::approve(&BALTHAZAR, asset, &STAKING_CONTRACT_ADDRESS, amount);
        staking::stake(&BALTHAZAR, 3_000, BOB.to_raw_vec(), amount);
    });

    // ensure stake reported to tellor pallet on oracle consumer parachain
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
        // create asset
        parachains::evm::create_xctrb_asset();
        // deploy contracts
        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
        // init contracts with addresses
        staking::init(&GOVERNANCE_CONTRACT_ADDRESS);
    });

    // register oracle consumer parachain with contracts on evm parachain via tellor pallet
    OracleConsumerParachain::execute_with(|| {
        parachains::oracle_consumer::register(2_000);
    });

    // mint, approve and stake trb in staking contract for oracle consumer parachain
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
        use oracle_consumer_runtime::System;
        System::assert_has_event(
            tellor::Event::NewStakerReported {
                staker: BOB,
                amount: amount.into(),
                address: H160(BALTHAZAR),
            }
            .into(),
        );
        parachains::oracle_consumer::submit_value(BOB, b"hello tellor", b"hey!")
    });
}

#[test]
fn disputes_value() {
    init_tracing();
    Network::reset();

    // create trb asset and deploy contracts
    EvmParachain::execute_with(|| {
        use parachains::{evm::contracts::*, evm::ALITH};
        // create asset
        parachains::evm::create_xctrb_asset();
        // deploy contracts
        registry::deploy();
        staking::deploy(&REGISTRY_CONTRACT_ADDRESS, &XCTRB_ADDRESS);
        governance::deploy(&REGISTRY_CONTRACT_ADDRESS, &ALITH);
        // init contracts with addresses
        staking::init(&GOVERNANCE_CONTRACT_ADDRESS);
        governance::init(&STAKING_CONTRACT_ADDRESS);
    });

    // register oracle consumer parachain with contracts on evm parachain via tellor pallet
    OracleConsumerParachain::execute_with(|| {
        parachains::oracle_consumer::register(2_000);
    });

    // mint, approve and stake trb in staking contract for oracle consumer parachain
    let amount = <oracle_consumer_runtime::Runtime as tellor::Config>::MinimumStakeAmount::get();
    EvmParachain::execute_with(|| {
        use parachains::evm::contracts::staking;
        let asset = u128::from_be_bytes(XCTRB_ADDRESS[4..].try_into().unwrap());
        staking::mint(asset, &BALTHAZAR, amount);
        staking::approve(&BALTHAZAR, asset, &STAKING_CONTRACT_ADDRESS, amount);
        staking::stake(&BALTHAZAR, 3_000, BOB.to_raw_vec(), amount);
    });

    // submit value to oracle consumer parachain and then begin dispute of reported value
    let (query_id, timestamp) = OracleConsumerParachain::execute_with(|| {
        use oracle_consumer_runtime::{Runtime, RuntimeOrigin, System, Tellor};
        // submit value
        let (query_id, timestamp) =
            parachains::oracle_consumer::submit_value(BOB, b"hello tellor", b"hey!");
        // begin dispute
        assert_ok!(Tellor::begin_dispute(
            RuntimeOrigin::signed(DAVE),
            query_id,
            timestamp,
            Some(DOROTHY.into())
        ));
        let dispute_id = Keccak256::hash(&ethabi::encode(&[
            ethabi::Token::Uint(<Runtime as tellor::Config>::ParachainId::get().into()),
            ethabi::Token::FixedBytes(query_id.0.into()),
            ethabi::Token::Uint(timestamp.into()),
        ]));
        System::assert_has_event(
            tellor::Event::NewDispute {
                dispute_id,
                query_id,
                timestamp,
                reporter: BOB,
            }
            .into(),
        );
        System::assert_has_event(
            tellor::Event::NewDisputeSent {
                para_id: 2_000,
                contract_address: GOVERNANCE_CONTRACT_ADDRESS.into(),
            }
            .into(),
        );
        (query_id, timestamp)
    });

    // ensure governance contract called and events emitted on evm parachain
    EvmParachain::execute_with(|| {
        use parachains::evm::*;
        // ensure governance contract called (via pallet derivative account on evm parachain)
        contracts::governance::assert_executed(&PALLET_DERIVATIVE_ACCOUNT);
        // ensure ParachainReporterSlashed event emitted by parachain staking contract
        contracts::staking::assert_parachain_reporter_slashed_event(
            3_000,
            &BALTHAZAR,
            &GOVERNANCE_CONTRACT_ADDRESS,
            amount,
        );
        // ensure NewParachainDispute event emitted by parachain governance contract
        contracts::governance::assert_new_parachain_dispute_event(
            3_000,
            query_id.0.to_vec(),
            timestamp,
            &BALTHAZAR,
        );
    });

    // ensure slash reported to tellor pallet on oracle consumer parachain
    OracleConsumerParachain::execute_with(|| {
        use oracle_consumer_runtime::System;
        System::assert_has_event(
            tellor::Event::SlashReported {
                reporter: BOB,
                amount: amount.into(),
            }
            .into(),
        );
    });
}
