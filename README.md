# CosmWasm Starter Pack

This is a template to build smart contracts in Rust to run inside a
[Cosmos SDK](https://github.com/cosmos/cosmos-sdk) module on all chains that enable it.
To understand the framework better, please read the overview in the
[cosmwasm repo](https://github.com/CosmWasm/cosmwasm/blob/master/README.md),
and dig into the [cosmwasm docs](https://www.cosmwasm.com).
This assumes you understand the theory and just want to get coding.

## Creating a new repo from template

Assuming you have a recent version of Rust and Cargo installed
(via [rustup](https://rustup.rs/)),
then the following should get you a new repo to start a contract:

Install [cargo-generate](https://github.com/ashleygwilliams/cargo-generate) and cargo-run-script.
Unless you did that before, run this line now:

```sh
cargo install cargo-generate --features vendored-openssl
cargo install cargo-run-script
```

Now, use it to create your new contract.
Go to the folder in which you want to place it and run:

**Latest**

```sh
cargo generate --git https://github.com/CosmWasm/cw-template.git --name PROJECT_NAME
```

For cloning minimal code repo:

```sh
cargo generate --git https://github.com/CosmWasm/cw-template.git --name PROJECT_NAME -d minimal=true
```

You will now have a new folder called `PROJECT_NAME` (I hope you changed that to something else)
containing a simple working contract and build system that you can customize.

## Create a Repo

After generating, you have a initialized local git repo, but no commits, and no remote.
Go to a server (eg. github) and create a new upstream repo (called `YOUR-GIT-URL` below).
Then run the following:

```sh
# this is needed to create a valid Cargo.lock file (see below)
cargo check
git branch -M main
git add .
git commit -m 'Initial Commit'
git remote add origin YOUR-GIT-URL
git push -u origin main
```

## CI Support

We have template configurations for both [GitHub Actions](.github/workflows/Basic.yml)
and [Circle CI](.circleci/config.yml) in the generated project, so you can
get up and running with CI right away.

One note is that the CI runs all `cargo` commands
with `--locked` to ensure it uses the exact same versions as you have locally. This also means
you must have an up-to-date `Cargo.lock` file, which is not auto-generated.
The first time you set up the project (or after adding any dep), you should ensure the
`Cargo.lock` file is updated, so the CI will test properly. This can be done simply by
running `cargo check` or `cargo unit-test`.

## Using your project

Once you have your custom repo, you should check out [Developing](./Developing.md) to explain
more on how to run tests and develop code. Or go through the
[online tutorial](https://docs.cosmwasm.com/) to get a better feel
of how to develop.

[Publishing](./Publishing.md) contains useful information on how to publish your contract
to the world, once you are ready to deploy it on a running blockchain. And
[Importing](./Importing.md) contains information about pulling in other contracts or crates
that have been published.

Please replace this README file with information about your specific project. You can keep
the `Developing.md` and `Publishing.md` files as useful references, but please set some
proper description in the README.

Airdrop Contract Documentation
Overview
The airdrop contract is a CosmWasm smart contract designed to distribute tokens (denominated as ukopi) to a predefined set of addresses over a vesting period. It includes gamification mechanics to incentivize participation: as fewer users claim their rewards, the remaining unclaimed users receive a larger share of the total reward pool. The contract supports administrative controls, dynamic reward calculations, and robust error handling, making it suitable for token distribution on the Kopi blockchain.
Key Features

Vesting Schedule: Tokens are claimable linearly between from_timestamp and to_timestamp.
Dynamic Rewards: The reward per user increases as users claim, redistributing the pool among remaining active users.
Admin Controls: The owner can update configurations, add claim addresses, and claim unclaimed funds.
Error Handling: Custom errors for unauthorized actions, insufficient balance, and invalid parameters.
Migration Support: Supports upgrading from an older state schema.

Contract Structure
The contract is implemented in Rust using the cosmwasm-std and cw-storage-plus libraries. Key files include:

state.rs: Defines State, OldState, and UserInfo structs for contract and user data.
msg.rs: Specifies messages for instantiation, execution, queries, and migration.
error.rs: Defines custom errors (Unauthorized, FullyClaimed, InsufficientBalance, InvalidRewardPool).
contract.rs: Implements core logic for instantiation, execution, queries, and migration.
integration_tests.rs: Contains tests for contract functionality.

Instantiation
The contract is instantiated with an InstantiateMsg that sets up the initial state:
pub struct InstantiateMsg {
    pub owner: Addr,                     // Admin address
    pub claim_drop_addrs: Vec<String>,   // List of addresses eligible to claim
    pub from_timestamp: u64,             // Start of vesting period (Unix timestamp)
    pub to_timestamp: u64,               // End of vesting period (Unix timestamp)
    pub claim_amount: u64,               // Initial claim amount per user (in ukopi)
    pub denom: String,                   // Token denomination (e.g., "ukopi")
}

Behavior:

Initializes the State with the provided parameters, setting num_active_users to the number of claim_drop_addrs and total_reward_pool to claim_amount * num_active_users.
Sets each user’s UserInfo with total_claimable_amount = claim_amount and has_fully_claimed = false.
Funds the contract with sufficient tokens (e.g., total_reward_pool) via a BankMsg::Send during instantiation.

Example:
{
  "owner": "kopi1admin",
  "claim_drop_addrs": ["kopi1user1", "kopi1user2", "kopi1user3"],
  "from_timestamp": 1697059200,
  "to_timestamp": 1697145600,
  "claim_amount": 10000,
  "denom": "ukopi"
}

Update Functions
The UpdateConfig execution message allows the owner to modify contract parameters:
ExecuteMsg::UpdateConfig {
    owner: Option<Addr>,              // New owner address
    claim_drop_addrs: Option<Vec<String>>, // New list of claim addresses
    from_timestamp: Option<u64>,       // New vesting start
    to_timestamp: Option<u64>,         // New vesting end
    claim_amount: Option<u64>,         // New base claim amount
    airdrop_has_ended: Option<bool>,   // Toggle airdrop status
    total_reward_pool: Option<Uint128>, // New total reward pool
}

Behavior:

Only the owner can call this function; otherwise, Unauthorized is returned.
Updates fields if provided, recalculating num_active_users and total_reward_pool when claim_drop_addrs or claim_amount changes.
Validates total_reward_pool to ensure it doesn’t exceed the contract’s balance (InsufficientBalance) and is sufficient for remaining claims (InvalidRewardPool).

Example:Increase the reward pool to 60,000 ukopi:
{
  "update_config": {
    "total_reward_pool": "60000"
  }
}

Admin Functions
The contract provides two admin-only functions:

AddClaimDropAddrs:
ExecuteMsg::AddClaimDropAddrs { multiple_addrs: Vec<String> }


Adds new addresses to claim_drop_addrs.
Initializes each new user’s UserInfo with total_claimable_amount = claim_amount and has_fully_claimed = false.
Increments num_active_users and increases total_reward_pool by claim_amount per new address.
Requires owner authorization (Unauthorized if not owner).
Requires contract funding to cover the increased total_reward_pool.


ClaimUnclaimed:
ExecuteMsg::ClaimUnclaimed { address: String }


Allows the owner to claim remaining contract balance after the airdrop ends.
Sets airdrop_has_ended = true, num_active_users = 0, and total_reward_pool = 0.
Requires owner authorization (Unauthorized if not owner).
Fails if the contract balance is zero (Insufficient contract balance).



Token Claim Amount Calculation
Users claim tokens via the Claim message:
ExecuteMsg::Claim {}

Calculation Logic:

Eligibility: The user must be in claim_drop_addrs, not has_fully_claimed, and the current time must be between from_timestamp and to_timestamp. Otherwise, errors like No claim for address, FullyClaimed, or Claim not allowed are returned.
Vesting:
Before from_timestamp: Claims fail.
Between from_timestamp and to_timestamp: Claims are linearly vested based on elapsed time. For example, at 50% of the vesting period, 50% of the user’s share is claimable.
After to_timestamp: Users can claim their full remaining share.


Dynamic Amount: The claimable amount per user is total_reward_pool / num_active_users, adjusted for vesting. As users claim their full share, num_active_users decreases, increasing the share for remaining users.
Balance Check: Claims fail if the contract’s balance is insufficient (InsufficientBalance).

Example:

Initial state: 3 users, total_reward_pool = 30,000, num_active_users = 3, per-user amount = 10,000.
USER1 claims 5,000 (50% vested): total_reward_pool = 25,000, num_active_users = 2, per-user amount = 12,500.
USER2 claims 12,500: total_reward_pool = 12,500, num_active_users = 1, per-user amount = 12,500.
USER3 claims 12,500, ending the airdrop.

Gamification Mechanics
The contract incentivizes participation by redistributing the reward pool among remaining active users:

num_active_users: Tracks users who haven’t fully claimed (has_fully_claimed = false). Decrements when a user claims their full share (either by reaching their current per-user amount or after to_timestamp).
total_reward_pool: Represents the total tokens available. Decrements by the claimed amount when a user becomes fully claimed.
Dynamic Rewards: The per-user claim amount is total_reward_pool / num_active_users. As num_active_users decreases, unclaimed users get a larger share, incentivizing early participation to secure rewards before the pool shrinks.

Example:

Initial: 3 users, total_reward_pool = 30,000, per-user = 10,000.
After USER1 claims fully: 2 users, total_reward_pool = 20,000, per-user = 10,000.
After USER2 claims fully: 1 user, total_reward_pool = 10,000, per-user = 10,000.

Integration Tests
The integration_tests.rs file contains comprehensive tests using the cw_multi_test framework:

instantiate_and_query_config: Verifies correct initialization of num_active_users, total_reward_pool, and other config fields.
claim_before_vesting_fails: Ensures claims fail before from_timestamp.
claim_during_vesting: Tests partial claims and dynamic amount increases (e.g., from 10,000 to 12,500 per user).
claim_after_vesting: Tests full claims, FullyClaimed errors, and final user receiving the remaining pool.
add_claim_drop_addrs: Verifies adding new users updates num_active_users and total_reward_pool.
update_config_total_reward_pool: Tests increasing total_reward_pool and its effect on per-user amounts.
update_config_insufficient_balance: Tests InsufficientBalance error for unfunded pool updates.
update_config_invalid_reward_pool: Tests InvalidRewardPool error for low pool values.
claim_unclaimed: Verifies unclaimed funds collection and airdrop termination.
migrate_from_old_state: Tests migration from OldState, ensuring new fields are set.

Setup:

Uses a mock app with funded admin and user accounts.
Simulates vesting periods by advancing block time.
Funds the contract during instantiation and admin actions.

Error Handling
The contract defines custom errors in error.rs:

Unauthorized: Triggered for non-owner actions (e.g., UpdateConfig, AddClaimDropAddrs).
FullyClaimed: Returned when a user with has_fully_claimed = true tries to claim.
InsufficientBalance: Triggered when the contract’s balance is too low for claims or pool updates.
InvalidRewardPool: Returned when total_reward_pool is set below the minimum required for remaining claims.
Std: Wraps standard errors (e.g., invalid vesting period, no claim for address).

Errors provide detailed messages (e.g., required vs. available balance) for debugging.
Migration
The contract supports migration from an older state (OldState) via the migrate function:

Converts OldState to State, setting denom = "ukopi", num_active_users = claim_drop_addrs.len(), and total_reward_pool = claim_amount * num_active_users.
Tested in migrate_from_old_state to ensure compatibility.

Funding Considerations
The contract assumes sufficient funding for total_reward_pool:

Instantiation: Must include tokens (e.g., total_reward_pool amount) via BankMsg::Send.
AddClaimDropAddrs: Requires additional funding for new users.
UpdateConfig: Increasing total_reward_pool requires contract balance to cover the new amount.

Recommendation: Add an ExecuteMsg::FundContract to allow the owner to deposit tokens explicitly, ensuring InsufficientBalance errors are avoided.
Usage Example

Deploy:
Instantiate with InstantiateMsg, funding the contract with 30,000 ukopi for 3 users.


Claim:
Users call Claim during vesting to receive partial amounts or post-vesting for full shares.


Admin Actions:
Owner adds new users with AddClaimDropAddrs, funding the increased pool.
Owner updates total_reward_pool with UpdateConfig to boost rewards.


End Airdrop:
Owner calls ClaimUnclaimed to collect remaining tokens, setting airdrop_has_ended = true.



Security Considerations

Access Control: Only the owner can call admin functions, enforced by Unauthorized checks.
Balance Validation: InsufficientBalance ensures claims and pool updates don’t exceed contract funds.
Vesting Enforcement: Prevents premature claims and ensures linear vesting.
Safe Arithmetic: Uses saturating_sub to prevent underflows in num_active_users and total_reward_pool.

Future Improvements

Funding Message: Add ExecuteMsg::FundContract for explicit token deposits.
Precise Pool Validation: Enhance InvalidRewardPool check to sum remaining claims.
Query Enhancements: Add queries for total claimed amount or active user list.
Event Emissions: Emit detailed events for claims, pool updates, and user status changes.

This documentation provides a comprehensive guide to the airdrop contract’s functionality, usage, and testing. For further details, refer to the source code and integration tests.
