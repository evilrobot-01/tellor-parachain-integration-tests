use polkadot_primitives::runtime_api::runtime_decl_for_parachain_host::ParachainHostV4;
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

mod parachains;
mod relay_chain;
#[cfg(test)]
mod tests;

decl_test_network! {
    pub struct Network {
        relay_chain = Rococo,
        parachains = vec![
            (1_000, AssetReserveParachain),
            (2_000, EvmParachain),
            (3_000, OracleConsumerParachain),
        ],
    }
}

decl_test_relay_chain! {
    pub struct Rococo {
        Runtime = rococo_runtime::Runtime,
        XcmConfig = rococo_runtime::xcm_config::XcmConfig,
        new_ext = relay_chain::new_ext(),
    }
}

decl_test_parachain! {
    pub struct AssetReserveParachain {
        Runtime = statemine_runtime::Runtime,
        RuntimeOrigin = statemine_runtime::RuntimeOrigin,
        XcmpMessageHandler = statemine_runtime::XcmpQueue,
        DmpMessageHandler = statemine_runtime::DmpQueue,
        new_ext = parachains::asset_reserve::new_ext(1_000),
    }
}

decl_test_parachain! {
    pub struct EvmParachain {
        Runtime = moonbase_runtime::Runtime,
        RuntimeOrigin = moonbase_runtime::RuntimeOrigin,
        XcmpMessageHandler = moonbase_runtime::XcmpQueue,
        DmpMessageHandler = moonbase_runtime::DmpQueue,
        new_ext = parachains::evm::new_ext(2_000),
    }
}

decl_test_parachain! {
    pub struct OracleConsumerParachain {
        Runtime = oracle_consumer_runtime::Runtime,
        RuntimeOrigin = oracle_consumer_runtime::RuntimeOrigin,
        XcmpMessageHandler = oracle_consumer_runtime::XcmpQueue,
        DmpMessageHandler = oracle_consumer_runtime::DmpQueue,
        new_ext = parachains::oracle_consumer::new_ext(3_000),
    }
}
