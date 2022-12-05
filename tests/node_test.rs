use gclient::{EventProcessor, GearApi, Result};
use gstd::Encode;
use staking::io::InitStaking;

#[cfg(debug_assertions)]
const PATH: &str = "./target/wasm32-unknown-unknown/debug/staking.opt.wasm";

#[cfg(not(debug_assertions))]
const PATH: &str = "./target/wasm32-unknown-unknown/release/staking.opt.wasm";

const USERS: &[u64] = &[1, 2, 3, 4, 5, 6, 7, 8];

#[tokio::test]
async fn init() -> Result<()> {
    let api = GearApi::dev().await?;

    let mut listener = api.subscribe().await?; // Subscribing for events.

    // Checking that blocks still running.
    assert!(listener.blocks_running().await?);

    let staking = InitStaking {
        staking_token_address: USERS[1].into(),
        reward_token_address: USERS[2].into(),
        distribution_time: 10000,
        reward_total: 1000,
    };

    let staking_payload = staking.encode();

    let gas_info = api
        .calculate_upload_gas(
            None,
            gclient::code_from_os(PATH)?,
            staking_payload.clone(),
            0,
            true,
            None,
        )
        .await?;

    let (message_id, _program_id, _hash) = api
        .upload_program_bytes_by_path(
            PATH,
            gclient::bytes_now(),
            staking_payload,
            gas_info.min_limit,
            0,
        )
        .await?;

    assert!(listener.message_processed(message_id).await?.succeed());

    Ok(())
}
