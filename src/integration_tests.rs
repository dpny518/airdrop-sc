#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, migrate, query};
    use crate::msg::{
        ClaimedAmountResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    };
    use crate::state::OldState;
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    use cw_storage_plus::Item;

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
        Box::new(contract)
    }

    const ADMIN: &str = "admin";
    const USER1: &str = "user1";
    const USER2: &str = "user2";
    const USER3: &str = "user3";
    const NATIVE_DENOM: &str = "ukopi";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, api, storage| {
            // Initialize balances for users and admin
            router
                .bank
                .init_balance(
                    storage,
                    &api.addr_make(ADMIN),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1_000_000),
                    }],
                )
                .unwrap();
            router
                .bank
                .init_balance(
                    storage,
                    &api.addr_make(USER1),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    }],
                )
                .unwrap();
            router
                .bank
                .init_balance(
                    storage,
                    &api.addr_make(USER2),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    }],
                )
                .unwrap();
            router
                .bank
                .init_balance(
                    storage,
                    &api.addr_make(USER3),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    }],
                )
                .unwrap();
        })
    }

    fn proper_instantiate() -> (App, Addr) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());

        let msg = InstantiateMsg {
            owner: app.api().addr_make(ADMIN),
            claim_drop_addrs: vec![
                app.api().addr_make(USER1).to_string(),
                app.api().addr_make(USER2).to_string(),
                app.api().addr_make(USER3).to_string(),
            ],
            from_timestamp: app.block_info().time.seconds() + 1000, // Start in 1000 seconds
            to_timestamp: app.block_info().time.seconds() + 2000, // End in 2000 seconds
            claim_amount: 10_000, // 10,000 ukopi per user initially
            denom: NATIVE_DENOM.to_string(),
        };

        // Fund the contract
        let contract_addr = app
            .instantiate_contract(
                cw_template_id,
                app.api().addr_make(ADMIN),
                &msg,
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(30_000), // Fund for 3 users
                }],
                "airdrop",
                None,
            )
            .unwrap();

        (app, contract_addr)
    }

    #[test]
    fn instantiate_and_query_config() {
        let (mut app, contract_addr) = proper_instantiate();

        // Query config
        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr, &QueryMsg::GetConfig {})
            .unwrap();
        assert_eq!(config.owner, app.api().addr_make(ADMIN));
        assert_eq!(config.denom, NATIVE_DENOM);
        assert_eq!(config.claim_amount, Uint128::new(10_000));
        assert_eq!(config.num_active_users, 3);
        assert_eq!(config.total_reward_pool, Uint128::new(30_000));
        assert!(!config.airdrop_has_ended);
    }

    #[test]
    fn claim_before_vesting_fails() {
        let (mut app, contract_addr) = proper_instantiate();

        // Try to claim before vesting starts
        let err = app
            .execute_contract(
                app.api().addr_make(USER1),
                contract_addr.clone(),
                &ExecuteMsg::Claim {},
                &[],
            )
            .unwrap_err();
        assert_eq!(
            err.root_cause().to_string(),
            "Generic error: Claim not allowed"
        );
    }

    #[test]
    fn claim_during_vesting() {
        let (mut app, contract_addr) = proper_instantiate();

        // Advance time to halfway through vesting (1500 seconds after start)
        app.update_block(|block| {
            block.time = block.time.plus_seconds(1500);
        });

        // USER1 claims (50% vested, initial share = 10,000)
        let resp = app
            .execute_contract(
                app.api().addr_make(USER1),
                contract_addr.clone(),
                &ExecuteMsg::Claim {},
                &[],
            )
            .unwrap();
        assert_eq!(
            resp.events
                .iter()
                .find(|e| e.ty == "wasm")
                .unwrap()
                .attributes
                .iter()
                .find(|a| a.key == "amount")
                .unwrap()
                .value,
            "5000" // 50% of 10,000
        );

        // Query USER1's claim status
        let claim_info: ClaimedAmountResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr,
                &QueryMsg::GetClaimedAmount {
                    address: app.api().addr_make(USER1),
                },
            )
            .unwrap();
        assert_eq!(claim_info.claimed_amount, Uint128::new(5000));
        assert_eq!(claim_info.current_per_user_amount, Uint128::new(10_000)); // Still 30,000 / 3

        // USER2 claims, pool now 30,000 - 5,000 = 25,000, active users = 2
        app.execute_contract(
            app.api().addr_make(USER2),
            contract_addr.clone(),
            &ExecuteMsg::Claim {},
            &[],
        )
        .unwrap();

        // Query USER3's claim status (unclaimed, should see larger share)
        let claim_info: ClaimedAmountResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr,
                &QueryMsg::GetClaimedAmount {
                    address: app.api().addr_make(USER3),
                },
            )
            .unwrap();
        assert_eq!(claim_info.claimed_amount, Uint128::zero());
        assert_eq!(claim_info.current_per_user_amount, Uint128::new(12_500)); // 25,000 / 2
        assert_eq!(claim_info.current_claimable_amount, Uint128::new(6250)); // 50% of 12,500
    }

    #[test]
    fn claim_after_vesting() {
        let (mut app, contract_addr) = proper_instantiate();

        // Advance time past vesting end
        app.update_block(|block| {
            block.time = block.time.plus_seconds(3000);
        });

        // USER1 claims full share
        app.execute_contract(
            app.api().addr_make(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Claim {},
            &[],
        )
        .unwrap();

        // Query USER1's claim status
        let claim_info: ClaimedAmountResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr,
                &QueryMsg::GetClaimedAmount {
                    address: app.api().addr_make(USER1),
                },
            )
            .unwrap();
        assert_eq!(claim_info.claimed_amount, Uint128::new(10_000));
        assert!(claim_info.current_claimable_amount.is_zero());

        // USER1 tries to claim again (should fail)
        let err = app
            .execute_contract(
                app.api().addr_make(USER1),
                contract_addr.clone(),
                &ExecuteMsg::Claim {},
                &[],
            )
            .unwrap_err();
        assert_eq!(err.root_cause().to_string(), "User has already fully claimed their rewards");

        // USER2 claims (pool now 20,000, active users = 2)
        app.execute_contract(
            app.api().addr_make(USER2),
            contract_addr.clone(),
            &ExecuteMsg::Claim {},
            &[],
        )
        .unwrap();

        // USER3 claims (should get full remaining share: 20,000 / 1 = 20,000)
        let resp = app
            .execute_contract(
                app.api().addr_make(USER3),
                contract_addr.clone(),
                &ExecuteMsg::Claim {},
                &[],
            )
            .unwrap();
        assert_eq!(
            resp.events
                .iter()
                .find(|e| e.ty == "wasm")
                .unwrap()
                .attributes
                .iter()
                .find(|a| a.key == "amount")
                .unwrap()
                .value,
            "20000"
        );
    }

    #[test]
    fn add_claim_drop_addrs() {
        let (mut app, contract_addr) = proper_instantiate();

        // Add USER4
        let new_user = app.api().addr_make("user4").to_string();
        app.execute_contract(
            app.api().addr_make(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::AddClaimDropAddrs {
                multiple_addrs: vec![new_user.clone()],
            },
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(10_000), // Fund for new user
            }],
        )
        .unwrap();

        // Query config
        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr, &QueryMsg::GetConfig {})
            .unwrap();
        assert_eq!(config.num_active_users, 4);
        assert_eq!(config.total_reward_pool, Uint128::new(40_000));

        // Query USER4's claim status
        let claim_info: ClaimedAmountResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr,
                &QueryMsg::GetClaimedAmount {
                    address: app.api().addr_make("user4"),
                },
            )
            .unwrap();
        assert_eq!(claim_info.current_per_user_amount, Uint128::new(10_000));
    }

    #[test]
    fn update_config_total_reward_pool() {
        let (mut app, contract_addr) = proper_instantiate();

        // Update total_reward_pool to 60,000 (requires funding)
        app.execute_contract(
            app.api().addr_make(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::UpdateConfig {
                owner: None,
                claim_drop_addrs: None,
                from_timestamp: None,
                to_timestamp: None,
                claim_amount: None,
                airdrop_has_ended: None,
                total_reward_pool: Some(Uint128::new(60_000)),
            },
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(30_000), // Additional funding
            }],
        )
        .unwrap();

        // Query config
        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr, &QueryMsg::GetConfig {})
            .unwrap();
        assert_eq!(config.total_reward_pool, Uint128::new(60_000));
        assert_eq!(config.num_active_users, 3);
        assert_eq!(config.claim_amount, Uint128::new(10_000));

        // Query USER1's claim status (should see larger share)
        let claim_info: ClaimedAmountResponse = app
            .wrap()
            .query_wasm_smart(
                &contract_addr,
                &QueryMsg::GetClaimedAmount {
                    address: app.api().addr_make(USER1),
                },
            )
            .unwrap();
        assert_eq!(claim_info.current_per_user_amount, Uint128::new(20_000)); // 60,000 / 3
    }

    #[test]
    fn update_config_insufficient_balance() {
        let (mut app, contract_addr) = proper_instantiate();

        // Try to update total_reward_pool to 100,000 without sufficient funding
        let err = app
            .execute_contract(
                app.api().addr_make(ADMIN),
                contract_addr.clone(),
                &ExecuteMsg::UpdateConfig {
                    owner: None,
                    claim_drop_addrs: None,
                    from_timestamp: None,
                    to_timestamp: None,
                    claim_amount: None,
                    airdrop_has_ended: None,
                    total_reward_pool: Some(Uint128::new(100_000)),
                },
                &[], // No additional funds
            )
            .unwrap_err();
        assert_eq!(
            err.root_cause().to_string(),
            "Insufficient contract balance: required 100000, available 30000"
        );
    }

    #[test]
    fn update_config_invalid_reward_pool() {
        let (mut app, contract_addr) = proper_instantiate();

        // Try to set total_reward_pool too low
        let err = app
            .execute_contract(
                app.api().addr_make(ADMIN),
                contract_addr.clone(),
                &ExecuteMsg::UpdateConfig {
                    owner: None,
                    claim_drop_addrs: None,
                    from_timestamp: None,
                    to_timestamp: None,
                    claim_amount: None,
                    airdrop_has_ended: None,
                    total_reward_pool: Some(Uint128::new(10_000)),
                },
                &[],
            )
            .unwrap_err();
        assert!(err
            .root_cause()
            .to_string()
            .starts_with("Invalid total reward pool: must be at least"));
    }

    #[test]
    fn claim_unclaimed() {
        let (mut app, contract_addr) = proper_instantiate();

        // Advance time past vesting end
        app.update_block(|block| {
            block.time = block.time.plus_seconds(3000);
        });

        // Claim unclaimed funds
        app.execute_contract(
            app.api().addr_make(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::ClaimUnclaimed {
                address: app.api().addr_make(ADMIN).to_string(),
            },
            &[],
        )
        .unwrap();

        // Query config
        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr, &QueryMsg::GetConfig {})
            .unwrap();
        assert!(config.airdrop_has_ended);
        assert_eq!(config.num_active_users, 0);
        assert_eq!(config.total_reward_pool, Uint128::zero());
    }

    #[test]
    fn migrate_from_old_state() {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());

        // Instantiate old state
        let old_state = OldState {
            owner: app.api().addr_make(ADMIN),
            claim_drop_addrs: vec![
                app.api().addr_make(USER1).to_string(),
                app.api().addr_make(USER2).to_string(),
            ],
            from_timestamp: app.block_info().time.seconds() + 1000,
            to_timestamp: app.block_info().time.seconds() + 2000,
            claim_amount: 10_000,
            airdrop_has_ended: false,
        };
        Item::new("state")
            .save(app.storage_mut(), &old_state)
            .unwrap();

        // Migrate
        app.migrate_contract(
            app.api().addr_make(ADMIN),
            app.api().addr_make("contract"), // Simulated contract address
            &MigrateMsg {},
            cw_template_id,
        )
        .unwrap();

        // Query config
        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                app.api().addr_make("contract"),
                &QueryMsg::GetConfig {},
            )
            .unwrap();
        assert_eq!(config.num_active_users, 2);
        assert_eq!(config.total_reward_pool, Uint128::new(20_000));
        assert_eq!(config.denom, "ukopi");
    }
}