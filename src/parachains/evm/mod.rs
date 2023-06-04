use super::*;
use frame_support::assert_ok;
use moonbase_runtime::{
    asset_config::AssetRegistrarMetadata, xcm_config::AssetType, AssetManager, Runtime,
    RuntimeEvent, RuntimeOrigin, System, EVM,
};
use sp_runtime::app_crypto::sp_core::bytes::from_hex;
use sp_runtime::app_crypto::sp_core::H160;
use xcm::prelude::{GeneralIndex, PalletInstance, Parachain};
use xcm::v3::{Junctions, MultiLocation};

pub(crate) mod contracts;

pub(crate) const ALITH: [u8; 20] = [
    242, 79, 243, 169, 207, 4, 199, 29, 188, 148, 208, 181, 102, 247, 162, 123, 148, 86, 108, 172,
];
pub(crate) const BALTHAZAR: [u8; 20] = [
    60, 208, 167, 5, 162, 220, 101, 229, 177, 225, 32, 88, 150, 186, 162, 190, 138, 7, 198, 224,
];
const INITIAL_EVM_BALANCE: u128 = 100 * 10u128.saturating_pow(18);
pub(crate) const PALLET_DERIVATIVE_ACCOUNT: [u8; 20] = [
    42, 161, 229, 255, 198, 29, 21, 138, 248, 84, 250, 40, 179, 31, 184, 119, 34, 232, 59, 100,
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
            (ALITH.into(), INITIAL_EVM_BALANCE),
            (BALTHAZAR.into(), INITIAL_EVM_BALANCE),
            (PALLET_DERIVATIVE_ACCOUNT.into(), INITIAL_EVM_BALANCE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub(crate) fn create_xctrb() {
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
    System::assert_last_event(RuntimeEvent::AssetManager(
        pallet_asset_manager::Event::ForeignAssetRegistered {
            asset_id,
            asset: asset.clone(),
            metadata,
        },
    ));
    // set units per second
    let units_per_second = 100_000; // todo: size correctly
    assert_ok!(AssetManager::set_asset_units_per_second(
        RuntimeOrigin::root(),
        asset.clone(),
        units_per_second,
        5, // todo: size correctly
    ));
    System::assert_last_event(RuntimeEvent::AssetManager(
        pallet_asset_manager::Event::UnitsPerSecondChanged {
            asset_type: asset,
            units_per_second,
        },
    ));
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
