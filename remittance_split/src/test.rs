#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as AddressTrait, Events, Ledger, LedgerInfo},
    Address, Env, IntoVal, Symbol, TryFromVal, Val, Vec,
};

use testutils::{set_ledger_time, setup_test_env};

// Removed local set_time in favor of testutils::set_ledger_time

#[test]
fn test_initialize_split_succeeds() {
    setup_test_env!(env, RemittanceSplit, client, owner);

    let success = client.initialize_split(
        &owner, &0,  // nonce
        &50, // spending
        &30, // savings
        &15, // bills
        &5,  // insurance
    );

    assert_eq!(success, true);

    let config = client.get_config().unwrap();
    assert_eq!(config.owner, owner);
    assert_eq!(config.spending_percent, 50);
    assert_eq!(config.savings_percent, 30);
    assert_eq!(config.bills_percent, 15);
    assert_eq!(config.insurance_percent, 5);
}

#[test]
fn test_initialize_split_invalid_sum() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    let result = client.try_initialize_split(
        &owner, &0, // nonce
        &50, &50, &10, // Sums to 110
        &0,
    );
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidPercentages)));
}

#[test]
fn test_initialize_split_already_initialized() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    client.initialize_split(&owner, &0, &50, &30, &15, &5);
    // Second init should fail
    let result = client.try_initialize_split(&owner, &1, &50, &30, &15, &5);
    assert_eq!(result, Err(Ok(RemittanceSplitError::AlreadyInitialized)));
}

#[test]
fn test_update_split() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let success = client.update_split(&owner, &1, &40, &40, &10, &10);
    assert_eq!(success, true);

    let config = client.get_config().unwrap();
    assert_eq!(config.spending_percent, 40);
    assert_eq!(config.savings_percent, 40);
    assert_eq!(config.bills_percent, 10);
    assert_eq!(config.insurance_percent, 10);
}

#[test]
fn test_update_split_unauthorized() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    env.mock_all_auths();

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let result = client.try_update_split(&other, &0, &40, &40, &10, &10);
    assert_eq!(result, Err(Ok(RemittanceSplitError::Unauthorized)));
}

#[test]
fn test_calculate_split() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    // Test with 1000 units
    let amounts = client.calculate_split(&1000);

    // spending: 50% of 1000 = 500
    // savings: 30% of 1000 = 300
    // bills: 15% of 1000 = 150
    // insurance: remainder = 1000 - 500 - 300 - 150 = 50

    assert_eq!(amounts.get(0).unwrap(), 500);
    assert_eq!(amounts.get(1).unwrap(), 300);
    assert_eq!(amounts.get(2).unwrap(), 150);
    assert_eq!(amounts.get(3).unwrap(), 50);
}

#[test]
fn test_calculate_split_rounding() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    // 33, 33, 33, 1 setup
    client.initialize_split(&owner, &0, &33, &33, &33, &1);

    // Total 100
    // 33% = 33
    // Remainder should go to last one (insurance) logic in contract:
    // insurance = total - spending - savings - bills
    // 100 - 33 - 33 - 33 = 1. Correct.

    let amounts = client.calculate_split(&100);
    assert_eq!(amounts.get(0).unwrap(), 33);
    assert_eq!(amounts.get(1).unwrap(), 33);
    assert_eq!(amounts.get(2).unwrap(), 33);
    assert_eq!(amounts.get(3).unwrap(), 1);
}

#[test]
fn test_calculate_split_zero_amount() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();
    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let result = client.try_calculate_split(&0);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidAmount)));
}

#[test]
fn test_calculate_complex_rounding() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();
    // 17, 19, 23, 41 (Primes summing to 100)
    client.initialize_split(&owner, &0, &17, &19, &23, &41);

    // Amount 1000
    // 17% = 170
    // 19% = 190
    // 23% = 230
    // 41% = 410
    // Sum = 1000. Perfect.
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 170);
    assert_eq!(amounts.get(1).unwrap(), 190);
    assert_eq!(amounts.get(2).unwrap(), 230);
    assert_eq!(amounts.get(3).unwrap(), 410);

    // Amount 3
    // 17% of 3 = 0
    // 19% of 3 = 0
    // 23% of 3 = 0
    // Remainder = 3 - 0 - 0 - 0 = 3. All goes to insurance.
    let tiny_amounts = client.calculate_split(&3);
    assert_eq!(tiny_amounts.get(0).unwrap(), 0);
    assert_eq!(tiny_amounts.get(3).unwrap(), 3);
}

#[test]
fn test_create_remittance_schedule_succeeds() {
    setup_test_env!(env, RemittanceSplit, client, owner);
    set_ledger_time(&env, 1000);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let schedule_id = client.create_remittance_schedule(&owner, &10000, &3000, &86400);
    assert_eq!(schedule_id, 1);

    let schedule = client.get_remittance_schedule(&schedule_id);
    assert!(schedule.is_some());
    let schedule = schedule.unwrap();
    assert_eq!(schedule.amount, 10000);
    assert_eq!(schedule.next_due, 3000);
    assert_eq!(schedule.interval, 86400);
    assert!(schedule.active);
}

#[test]
fn test_modify_remittance_schedule() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = <soroban_sdk::Address as AddressTrait>::generate(&env);

    env.mock_all_auths();
    set_time(&env, 1000);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let schedule_id = client.create_remittance_schedule(&owner, &10000, &3000, &86400);
    client.modify_remittance_schedule(&owner, &schedule_id, &15000, &4000, &172800);

    let schedule = client.get_remittance_schedule(&schedule_id).unwrap();
    assert_eq!(schedule.amount, 15000);
    assert_eq!(schedule.next_due, 4000);
    assert_eq!(schedule.interval, 172800);
}

#[test]
fn test_cancel_remittance_schedule() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = <soroban_sdk::Address as AddressTrait>::generate(&env);

    env.mock_all_auths();
    set_time(&env, 1000);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let schedule_id = client.create_remittance_schedule(&owner, &10000, &3000, &86400);
    client.cancel_remittance_schedule(&owner, &schedule_id);

    let schedule = client.get_remittance_schedule(&schedule_id).unwrap();
    assert!(!schedule.active);
}

#[test]
fn test_get_remittance_schedules() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = <soroban_sdk::Address as AddressTrait>::generate(&env);

    env.mock_all_auths();
    set_time(&env, 1000);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    client.create_remittance_schedule(&owner, &10000, &3000, &86400);
    client.create_remittance_schedule(&owner, &5000, &4000, &172800);

    let schedules = client.get_remittance_schedules(&owner);
    assert_eq!(schedules.len(), 2);
}

#[test]
fn test_remittance_schedule_validation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = <soroban_sdk::Address as AddressTrait>::generate(&env);

    env.mock_all_auths();
    set_time(&env, 5000);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let result = client.try_create_remittance_schedule(&owner, &10000, &3000, &86400);
    assert!(result.is_err());
}

#[test]
fn test_remittance_schedule_zero_amount() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = <soroban_sdk::Address as AddressTrait>::generate(&env);

    env.mock_all_auths();
    set_time(&env, 1000);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let result = client.try_create_remittance_schedule(&owner, &0, &3000, &86400);
    assert!(result.is_err());
}
#[test]
fn test_initialize_split_events() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let events = env.events().all();
    let last_event = events.last().unwrap();

    // The event emitted is: env.events().publish((symbol_short!("split"), SplitEvent::Initialized), owner);
    assert_eq!(last_event.0, contract_id);

    let topics = &last_event.1;
    let topic0: Symbol = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    let topic1: SplitEvent = SplitEvent::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
    assert_eq!(topic0, symbol_short!("split"));
    assert_eq!(topic1, SplitEvent::Initialized);

    let data: Address = Address::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, owner);
}

#[test]
fn test_update_split_events() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    client.initialize_split(&owner, &0, &50, &30, &15, &5);
    client.update_split(&owner, &1, &40, &40, &10, &10);

    let events = env.events().all();
    // update_split publishes two events:
    // 1. (SPLIT_INITIALIZED,), event
    // 2. (symbol_short!("split"), SplitEvent::Updated), caller
    let last_event = events.last().unwrap();

    assert_eq!(last_event.0, contract_id);

    let topics = &last_event.1;
    let topic0: Symbol = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    let topic1: SplitEvent = SplitEvent::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
    assert_eq!(topic0, symbol_short!("split"));
    assert_eq!(topic1, SplitEvent::Updated);

    let data: Address = Address::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, owner);
}

#[test]
fn test_calculate_split_events() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let total_amount = 1000i128;
    client.calculate_split(&total_amount);

    let events = env.events().all();
    // calculate_split publishes two events:
    // 1. (SPLIT_CALCULATED,), event
    // 2. (symbol_short!("split"), SplitEvent::Calculated), total_amount
    let last_event = events.last().unwrap();

    assert_eq!(last_event.0, contract_id);

    let topics = &last_event.1;
    let topic0: Symbol = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    let topic1: SplitEvent = SplitEvent::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
    assert_eq!(topic0, symbol_short!("split"));
    assert_eq!(topic1, SplitEvent::Calculated);

    let data: i128 = i128::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, total_amount);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn test_update_split_non_owner_auth_failure() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &owner,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize_split",
                args: (&owner, 0u64, 50u32, 30u32, 15u32, 5u32).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize_split(&owner, &0, &50, &30, &15, &5);

    // Call as other without mocking auth, expecting panic
    client.update_split(&other, &0, &40, &40, &10, &10);
}

// ──────────────────────────────────────────────────────────────────────────
// Boundary tests for split percentages (#103)
// ──────────────────────────────────────────────────────────────────────────
// ──────────────────────────────────────────────────────────────────────────
// Boundary tests for split percentages (#103)
// ──────────────────────────────────────────────────────────────────────────

/// 100 % spending, all other categories zero.
#[test]
fn test_split_boundary_100_0_0_0() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    let ok = client.initialize_split(&owner, &0, &100, &0, &0, &0);
    assert!(ok);

    // get_split must return the exact percentages
    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 100);
    assert_eq!(split.get(1).unwrap(), 0);
    assert_eq!(split.get(2).unwrap(), 0);
    assert_eq!(split.get(3).unwrap(), 0);

    // calculate_split must allocate the entire amount to spending
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 1000);
    assert_eq!(amounts.get(1).unwrap(), 0);
    assert_eq!(amounts.get(2).unwrap(), 0);
    assert_eq!(amounts.get(3).unwrap(), 0);
}

/// 100 % savings, all other categories zero.
#[test]
fn test_split_boundary_0_100_0_0() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    let ok = client.initialize_split(&owner, &0, &0, &100, &0, &0);
    assert!(ok);

    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 0);
    assert_eq!(split.get(1).unwrap(), 100);
    assert_eq!(split.get(2).unwrap(), 0);
    assert_eq!(split.get(3).unwrap(), 0);

    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 0);
    assert_eq!(amounts.get(1).unwrap(), 1000);
    assert_eq!(amounts.get(2).unwrap(), 0);
    assert_eq!(amounts.get(3).unwrap(), 0);
}

/// 100 % bills, all other categories zero.
#[test]
fn test_split_boundary_0_0_100_0() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    let ok = client.initialize_split(&owner, &0, &0, &0, &100, &0);
    assert!(ok);

    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 0);
    assert_eq!(split.get(1).unwrap(), 0);
    assert_eq!(split.get(2).unwrap(), 100);
    assert_eq!(split.get(3).unwrap(), 0);

    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 0);
    assert_eq!(amounts.get(1).unwrap(), 0);
    assert_eq!(amounts.get(2).unwrap(), 1000);
    assert_eq!(amounts.get(3).unwrap(), 0);
}

/// 100 % insurance, all other categories zero.
#[test]
fn test_split_boundary_0_0_0_100() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    let ok = client.initialize_split(&owner, &0, &0, &0, &0, &100);
    assert!(ok);

    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 0);
    assert_eq!(split.get(1).unwrap(), 0);
    assert_eq!(split.get(2).unwrap(), 0);
    assert_eq!(split.get(3).unwrap(), 100);

    // Insurance gets the remainder: 1000 - 0 - 0 - 0 = 1000
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 0);
    assert_eq!(amounts.get(1).unwrap(), 0);
    assert_eq!(amounts.get(2).unwrap(), 0);
    assert_eq!(amounts.get(3).unwrap(), 1000);
}

/// Equal split: 25 / 25 / 25 / 25.
#[test]
fn test_split_boundary_25_25_25_25() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    let ok = client.initialize_split(&owner, &0, &25, &25, &25, &25);
    assert!(ok);

    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 25);
    assert_eq!(split.get(1).unwrap(), 25);
    assert_eq!(split.get(2).unwrap(), 25);
    assert_eq!(split.get(3).unwrap(), 25);

    // 25 % of 1000 = 250 for each category
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 250);
    assert_eq!(amounts.get(1).unwrap(), 250);
    assert_eq!(amounts.get(2).unwrap(), 250);
    assert_eq!(amounts.get(3).unwrap(), 250);
}

/// update_split with boundary percentages: change from a normal split
/// to 100/0/0/0, then to 25/25/25/25.
#[test]
fn test_update_split_boundary_percentages() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    env.mock_all_auths();

    // Start with a typical split
    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    // Update to 100/0/0/0
    let ok = client.update_split(&owner, &1, &100, &0, &0, &0);
    assert!(ok);

    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 100);
    assert_eq!(split.get(1).unwrap(), 0);
    assert_eq!(split.get(2).unwrap(), 0);
    assert_eq!(split.get(3).unwrap(), 0);

    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 1000);
    assert_eq!(amounts.get(1).unwrap(), 0);
    assert_eq!(amounts.get(2).unwrap(), 0);
    assert_eq!(amounts.get(3).unwrap(), 0);

    // Update again to 25/25/25/25
    let ok = client.update_split(&owner, &1, &25, &25, &25, &25);
    assert!(ok);

    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 25);
    assert_eq!(split.get(1).unwrap(), 25);
    assert_eq!(split.get(2).unwrap(), 25);
    assert_eq!(split.get(3).unwrap(), 25);

    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 250);
    assert_eq!(amounts.get(1).unwrap(), 250);
    assert_eq!(amounts.get(2).unwrap(), 250);
    assert_eq!(amounts.get(3).unwrap(), 250);
}

#[test]
fn test_update_split_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let caller = Address::generate(&env);

    let result = client.try_update_split(&caller, &0, &25, &25, &25, &25);
    assert_eq!(result, Err(Ok(RemittanceSplitError::NotInitialized)));

    let config = client.get_config();
    assert!(config.is_none());

    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 50);
    assert_eq!(split.get(1).unwrap(), 30);
    assert_eq!(split.get(2).unwrap(), 15);
    assert_eq!(split.get(3).unwrap(), 5);
}

// ──────────────────────────────────────────────────────────────────────────
// Request Hash Tests - Test Vectors for distribute_usdc Signing
// ──────────────────────────────────────────────────────────────────────────

/// Test that get_request_hash produces a deterministic 32-byte SHA-256 hash
#[test]
fn test_request_hash_deterministic() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    let usdc_contract = Address::generate(&env);
    let from = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    
    let request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: from.clone(),
        nonce: 0,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 1000i128,
        deadline: 2000u64,
    };
    
    // Hash the same request twice
    let hash1 = client.get_request_hash(&request);
    let hash2 = client.get_request_hash(&request);
    
    // Both hashes should be identical (deterministic)
    assert_eq!(hash1, hash2);
    // SHA-256 produces 32 bytes
    assert_eq!(hash1.len(), 32);
}

/// Test that changing any parameter changes the hash (no collisions)
#[test]
fn test_request_hash_changes_with_parameters() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    let usdc_contract = Address::generate(&env);
    let from = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    let other = Address::generate(&env);
    
    let base_request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: from.clone(),
        nonce: 0,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 1000i128,
        deadline: 2000u64,
    };
    
    let base_hash = client.get_request_hash(&base_request);
    
    // Test 1: Changing usdc_contract changes hash
    let mut req = base_request.clone();
    req.usdc_contract = other.clone();
    let hash = client.get_request_hash(&req);
    assert!(hash.ne(&base_hash), "Hash should change when usdc_contract changes");
    
    // Test 2: Changing from address changes hash
    let mut req = base_request.clone();
    req.from = other.clone();
    let hash = client.get_request_hash(&req);
    assert!(hash.ne(&base_hash), "Hash should change when from changes");
    
    // Test 3: Changing nonce changes hash
    let mut req = base_request.clone();
    req.nonce = 1;
    let hash = client.get_request_hash(&req);
    assert!(hash.ne(&base_hash), "Hash should change when nonce changes");
    
    // Test 4: Changing total_amount changes hash
    let mut req = base_request.clone();
    req.total_amount = 2000;
    let hash = client.get_request_hash(&req);
    assert!(hash.ne(&base_hash), "Hash should change when total_amount changes");
    
    // Test 5: Changing deadline changes hash
    let mut req = base_request.clone();
    req.deadline = 3000;
    let hash = client.get_request_hash(&req);
    assert!(hash.ne(&base_hash), "Hash should change when deadline changes");
    
    // Test 6: Changing spending account changes hash
    let mut req = base_request.clone();
    req.accounts.spending = other.clone();
    let hash = client.get_request_hash(&req);
    assert!(hash.ne(&base_hash), "Hash should change when spending account changes");
}

/// Test deadline validation: deadline must not be in the past
#[test]
fn test_distribute_usdc_deadline_expired() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    env.mock_all_auths();
    set_ledger_time(&env, 1000);
    
    let owner = Address::generate(&env);
    let usdc_contract = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    
    // Initialize contract
    client.initialize_split(&owner, &0, &50, &30, &15, &5);
    
    // Create request with deadline in the past (500 < 1000)
    let request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: owner.clone(),
        nonce: 0,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 1000i128,
        deadline: 500u64,  // Past deadline
    };
    
    let hash = client.get_request_hash(&request);
    let result = client.try_distribute_usdc_with_hash_and_deadline(&request, &hash);
    assert_eq!(result, Err(Ok(RemittanceSplitError::DeadlineExpired)));
}

/// Test deadline validation: deadline must not be too far in the future (MAX_DEADLINE_WINDOW_SECS = 3600)
#[test]
fn test_distribute_usdc_deadline_too_far() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    env.mock_all_auths();
    set_ledger_time(&env, 1000);
    
    let owner = Address::generate(&env);
    let usdc_contract = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    
    // Initialize contract
    client.initialize_split(&owner, &0, &50, &30, &15, &5);
    
    // Create request with deadline > MAX_DEADLINE_WINDOW_SECS from now
    let request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: owner.clone(),
        nonce: 0,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 1000i128,
        deadline: 1000 + 3600 + 1,  // 1 second more than allowed window
    };
    
    let hash = client.get_request_hash(&request);
    let result = client.try_distribute_usdc_with_hash_and_deadline(&request, &hash);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidDeadline)));
}

/// Test deadline validation: deadline must not be zero
#[test]
fn test_distribute_usdc_deadline_zero() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    env.mock_all_auths();
    set_ledger_time(&env, 1000);
    
    let owner = Address::generate(&env);
    let usdc_contract = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    
    // Initialize contract
    client.initialize_split(&owner, &0, &50, &30, &15, &5);
    
    // Create request with deadline = 0
    let request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: owner.clone(),
        nonce: 0,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 1000i128,
        deadline: 0,  // Invalid deadline
    };
    
    let hash = client.get_request_hash(&request);
    let result = client.try_distribute_usdc_with_hash_and_deadline(&request, &hash);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidDeadline)));
}

/// Test request hash mismatch: passing wrong hash should fail
#[test]
fn test_distribute_usdc_hash_mismatch() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    env.mock_all_auths();
    set_ledger_time(&env, 1000);
    
    let owner = Address::generate(&env);
    let usdc_contract = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    
    // Initialize contract
    client.initialize_split(&owner, &0, &50, &30, &15, &5);
    
    // Create valid request
    let request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: owner.clone(),
        nonce: 0,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 1000i128,
        deadline: 2000u64,
    };
    
    // Get correct hash and then create a wrong one
    let correct_hash = client.get_request_hash(&request);
    let mut wrong_hash = correct_hash.clone();
    // Flip a byte to create a different hash
    if wrong_hash.get(0).unwrap() != &0xFFu8 {
        wrong_hash.set(0, &(wrong_hash.get(0).unwrap() + 1));
    } else {
        wrong_hash.set(0, &(wrong_hash.get(0).unwrap() - 1));
    }
    
    let result = client.try_distribute_usdc_with_hash_and_deadline(&request, &wrong_hash);
    assert_eq!(result, Err(Ok(RemittanceSplitError::RequestHashMismatch)));
}

/// Test boundary: deadline exactly at MAX_DEADLINE_WINDOW_SECS should succeed
#[test]
fn test_distribute_usdc_deadline_at_boundary() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    env.mock_all_auths();
    set_ledger_time(&env, 1000);
    
    let owner = Address::generate(&env);
    let usdc_contract = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    
    // Initialize contract
    client.initialize_split(&owner, &0, &50, &30, &15, &5);
    
    // Create request with deadline exactly at MAX_DEADLINE_WINDOW_SECS boundary
    let request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: owner.clone(),
        nonce: 0,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 1000i128,
        deadline: 1000 + 3600,  // Exactly at 1 hour boundary
    };
    
    let hash = client.get_request_hash(&request);
    
    // This should pass deadline validation
    // (It will fail for other reasons like missing USDC balance, but not deadline)
    let result = client.try_distribute_usdc_with_hash_and_deadline(&request, &hash);
    
    // Should fail due to other reasons (e.g., balance), not deadline validation
    // We can't assert equality here since we didn't register USDC token,
    // but we can check it's not a DeadlineExpired or InvalidDeadline error
    match result {
        Err(Ok(RemittanceSplitError::DeadlineExpired)) => {
            panic!("Should not fail with DeadlineExpired");
        }
        Err(Ok(RemittanceSplitError::InvalidDeadline)) => {
            panic!("Should not fail with InvalidDeadline");
        }
        _ => {} // Any other result is acceptable for this boundary test
    }
}

/// Test that the same request always produces the same hash (cross-call consistency)
#[test]
fn test_request_hash_cross_call_consistency() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    
    let usdc_contract = Address::generate(&env);
    let from = Address::generate(&env);
    let spending = Address::generate(&env);
    let savings = Address::generate(&env);
    let bills = Address::generate(&env);
    let insurance = Address::generate(&env);
    
    let request = DistributeUsdcRequest {
        usdc_contract: usdc_contract.clone(),
        from: from.clone(),
        nonce: 42,
        accounts: AccountGroup {
            spending: spending.clone(),
            savings: savings.clone(),
            bills: bills.clone(),
            insurance: insurance.clone(),
        },
        total_amount: 12345i128,
        deadline: 9999u64,
    };
    
    // Call get_request_hash multiple times
    let hashes: Vec<_> = (0..5)
        .map(|_| client.get_request_hash(&request))
        .collect();
    
    // All hashes should be identical
    for hash in &hashes[1..] {
        assert_eq!(hash, &hashes[0], "Hash should be consistent across calls");
    }
}

fn set_time(env: &Env, timestamp: u64) {
    env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 20,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        max_tx_size: 0,
        min_temp_entry_ttl: 0,
        min_persistent_entry_ttl: 0,
        max_entry_ttl: 0,
    });
}

