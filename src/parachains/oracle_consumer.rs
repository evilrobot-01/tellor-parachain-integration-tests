use super::*;
use crate::relay_chain::*;

const INITIAL_BALANCE: u128 = 1_000_000_000_000;
const PALLET_ACCOUNT: [u8; 32] = [
    109, 111, 100, 108, 112, 121, 47, 116, 101, 108, 108, 114, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

pub(crate) fn new_ext(para_id: u32) -> sp_io::TestExternalities {
    use oracle_consumer_runtime::{Runtime, System};

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
            (PALLET_ACCOUNT.into(), INITIAL_BALANCE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
