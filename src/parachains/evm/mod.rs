use super::*;
use core::time::Duration;
use frame_support::{assert_ok, traits::UnixTime};
use moonbeam_runtime::{
    asset_config::AssetRegistrarMetadata, xcm_config::AssetType, AssetManager, EVMConfig,
    GenesisAccount, Precompiles, Runtime, RuntimeEvent, RuntimeOrigin, System, Timestamp, EVM,
};
use sp_runtime::{app_crypto::sp_core::bytes::from_hex, app_crypto::sp_core::H160};
use xcm::prelude::{GeneralIndex, PalletInstance, Parachain};
use xcm::v3::{Junctions, MultiLocation};

pub(crate) mod contracts;

pub(crate) const ALITH: [u8; 20] = [
    242, 79, 243, 169, 207, 4, 199, 29, 188, 148, 208, 181, 102, 247, 162, 123, 148, 86, 108, 172,
];
pub(crate) const BALTHAZAR: [u8; 20] = [
    60, 208, 167, 5, 162, 220, 101, 229, 177, 225, 32, 88, 150, 186, 162, 190, 138, 7, 198, 224,
];
#[allow(dead_code)]
pub(crate) const CHARLETH: [u8; 20] = [
    121, 141, 75, 169, 186, 240, 6, 78, 193, 158, 180, 240, 161, 164, 87, 133, 174, 157, 109, 252,
];
pub(crate) const DOROTHY: [u8; 20] = [
    119, 53, 57, 212, 172, 14, 120, 98, 51, 217, 10, 35, 54, 84, 204, 238, 38, 166, 19, 217,
];
pub(crate) const PALLET_DERIVATIVE_ACCOUNT: [u8; 20] = [
    38, 171, 121, 151, 207, 109, 83, 31, 237, 18, 178, 250, 107, 195, 207, 34, 72, 114, 65, 149,
];
pub(crate) const XCTRB_ADDRESS: [u8; 20] = [
    255, 255, 255, 255, 200, 190, 87, 122, 39, 148, 132, 67, 27, 148, 68, 104, 126, 195, 210, 174,
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
            (ALITH.into(), 2 * 10u128.saturating_pow(18)), // contract deployment
            (BALTHAZAR.into(), 2 * 10u128.saturating_pow(18)), // contract transactions
            (
                PALLET_DERIVATIVE_ACCOUNT.into(),
                1 * 10u128.saturating_pow(18), // required for xcm fees
            ),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    // set precompiles revert bytecode: https://github.com/PureStake/moonbeam/blob/a814fcf36a67f0f14f40afcd7d12fd4f3c5e775b/node/service/src/chain_spec/moonbeam.rs#L244
    let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];
    let evm_config = EVMConfig {
        // We need _some_ code inserted at the precompile address so evm will actually call the address
        accounts: Precompiles::used_addresses()
            .map(|addr| {
                (
                    addr.into(),
                    GenesisAccount {
                        nonce: Default::default(),
                        balance: Default::default(),
                        storage: Default::default(),
                        code: revert_bytecode.clone(),
                    },
                )
            })
            .collect(),
    };
    <pallet_evm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(&evm_config, &mut t)
        .unwrap();

    // set xcm version
    let xcm_config = moonbeam_runtime::PolkadotXcmConfig {
        safe_xcm_version: Some(3),
    };
    <pallet_xcm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(&xcm_config, &mut t)
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

pub(crate) fn advance_time(time_in_secs: u64) {
    let now = <Timestamp as UnixTime>::now();
    pallet_timestamp::Now::<Runtime>::set(
        (now + Duration::from_secs(time_in_secs)).as_millis() as u64
    );
}

pub(crate) fn create_xctrb_asset() {
    let asset = AssetType::Xcm(MultiLocation {
        parents: 1,
        interior: Junctions::X3(Parachain(1_000), PalletInstance(50), GeneralIndex(872)),
    });
    let metadata = AssetRegistrarMetadata {
        name: b"Tellor Tribute".to_vec(),
        symbol: b"xcTRB".to_vec(),
        decimals: 18,
        is_frozen: false,
    };
    // register asset
    assert_ok!(AssetManager::register_foreign_asset(
        RuntimeOrigin::root(),
        asset.clone(),
        metadata.clone(),
        1,
        true
    ));
    let asset_id = u128::from_be_bytes(XCTRB_ADDRESS[4..].try_into().unwrap());
    System::assert_last_event(
        pallet_asset_manager::Event::ForeignAssetRegistered {
            asset_id,
            asset: asset.clone(),
            metadata,
        }
        .into(),
    );
    // set units per second
    let units_per_second = 100_000; // todo: size correctly
    assert_ok!(AssetManager::set_asset_units_per_second(
        RuntimeOrigin::root(),
        asset.clone(),
        units_per_second,
        5, // todo: size correctly
    ));
    System::assert_last_event(
        pallet_asset_manager::Event::UnitsPerSecondChanged {
            asset_type: asset,
            units_per_second,
        }
        .into(),
    );
    // push revert code to EVM: https://github.com/PureStake/xcm-tools/blob/4a3dcdb49434bcc019106677d01be54f9f17b30b/scripts/xcm-asset-registrator.ts#L87-L107
    // required for transferFrom usage within a smart contract, especially to return revert specifics
    assert_ok!(System::set_storage(
        RuntimeOrigin::root(),
        vec![(
            from_hex("0x1da53b775b270400e7e61ed5cbc5a146ea70f53d5a3306ce02aaf97049cf181a8d25f78201571c23f3f5096d309394ecffffffffc8be577a279484431b9444687ec3d2ae").unwrap(),
            from_hex("0x1460006000fd").unwrap()
        )]
    ));
}
