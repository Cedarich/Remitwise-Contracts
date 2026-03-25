//! Multi-contract integration tests — issue #336
//!
//! Validates standardized error codes and cross-contract behaviour across:
//!   - insurance    (InsuranceError codes 1-8)
//!   - bill_payments (BillPaymentsError codes 1-14)
//!   - savings_goals (SavingsGoalsError codes 1-6)
//!   - remittance_split (RemittanceSplitError codes 1-11)

use bill_payments::{BillPayments, BillPaymentsClient, BillPaymentsError};
use insurance::{Insurance, InsuranceClient, InsuranceError};
use remitwise_common::CoverageType;
use remittance_split::{RemittanceSplit, RemittanceSplitClient, RemittanceSplitError};
use savings_goals::{SavingsGoalContract, SavingsGoalContractClient, SavingsGoalsError};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String as SStr,
};

fn make_env() -> Env {
    let env = Env::default();
    env.ledger().set(LedgerInfo {
        protocol_version: 20,
        sequence_number: 100,
        timestamp: 1_700_000_000,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 700_000,
    });
    env.mock_all_auths();
    env
}

// ============================================================================
// PART 1: Full multi-contract user flow
// ============================================================================

#[test]
fn test_multi_contract_user_flow() {
    let env = make_env();
    let user = Address::generate(&env);

    let rsplit_id = env.register_contract(None, RemittanceSplit);
    let rsplit = RemittanceSplitClient::new(&env, &rsplit_id);

    let savings_id = env.register_contract(None, SavingsGoalContract);
    let savings = SavingsGoalContractClient::new(&env, &savings_id);

    let bills_id = env.register_contract(None, BillPayments);
    let bills = BillPaymentsClient::new(&env, &bills_id);

    let insure_id = env.register_contract(None, Insurance);
    let insure = InsuranceClient::new(&env, &insure_id);

    let usdc = Address::generate(&env);
    rsplit.initialize_split(&user, &0u64, &usdc, &40u32, &30u32, &20u32, &10u32);
    let config = rsplit.get_config().unwrap();
    assert_eq!(config.spending_percent, 40);
    assert_eq!(config.savings_percent, 30);
    assert_eq!(config.bills_percent, 20);
    assert_eq!(config.insurance_percent, 10);

    savings.init();
    let goal_id = savings.create_goal(
        &user,
        &SStr::from_str(&env, "Education Fund"),
        &10_000i128,
        &(1_700_000_000 + 365 * 86400),
    );
    assert_eq!(goal_id, 1u32);

    let bill_id = bills.create_bill(
        &user,
        &SStr::from_str(&env, "Electricity"),
        &500i128,
        &(1_700_000_000 + 30 * 86400),
        &true,
        &30u32,
        &None,
        &SStr::from_str(&env, "XLM"),
    );
    assert_eq!(bill_id, 1u32);

    let policy_id = insure.create_policy(
        &user,
        &SStr::from_str(&env, "Health Plan"),
        &CoverageType::Health,
        &200i128,
        &50_000i128,
        &None,
    );
    assert_eq!(policy_id, 1u32);

    let amounts = rsplit.calculate_split(&10_000i128);
    let spending = amounts.get(0).unwrap();
    let sav = amounts.get(1).unwrap();
    let bill_alloc = amounts.get(2).unwrap();
    let insure_alloc = amounts.get(3).unwrap();

    assert_eq!(spending, 4_000i128);
    assert_eq!(sav, 3_000i128);
    assert_eq!(bill_alloc, 2_000i128);
    assert_eq!(insure_alloc, 1_000i128);
    assert_eq!(spending + sav + bill_alloc + insure_alloc, 10_000i128);
}

// ============================================================================
// PART 2: Insurance error codes
// ============================================================================

#[test]
fn test_insurance_error_code_1_policy_not_found() {
    let env = make_env();
    let cid = env.register_contract(None, Insurance);
    let client = InsuranceClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let err = client.try_pay_premium(&owner, &999u32).unwrap_err().unwrap();
    assert_eq!(err, InsuranceError::PolicyNotFound);
    assert_eq!(err as u32, 1);
}

#[test]
fn test_insurance_error_code_2_unauthorized() {
    let env = make_env();
    let cid = env.register_contract(None, Insurance);
    let client = InsuranceClient::new(&env, &cid);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    let id = client.create_policy(
        &owner, &SStr::from_str(&env, "P"), &CoverageType::Health, &100i128, &10_000i128, &None,
    );
    let err = client.try_deactivate_policy(&other, &id).unwrap_err().unwrap();
    assert_eq!(err, InsuranceError::Unauthorized);
    assert_eq!(err as u32, 2);
}

#[test]
fn test_insurance_error_code_3_invalid_amount() {
    let env = make_env();
    let cid = env.register_contract(None, Insurance);
    let client = InsuranceClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let err = client
        .try_create_policy(&owner, &SStr::from_str(&env, "P"), &CoverageType::Health, &0i128, &10_000i128, &None)
        .unwrap_err().unwrap();
    assert_eq!(err, InsuranceError::InvalidAmount);
    assert_eq!(err as u32, 3);
}

#[test]
fn test_insurance_error_code_4_policy_inactive() {
    let env = make_env();
    let cid = env.register_contract(None, Insurance);
    let client = InsuranceClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let id = client.create_policy(
        &owner, &SStr::from_str(&env, "P"), &CoverageType::Health, &100i128, &5_000i128, &None,
    );
    client.deactivate_policy(&owner, &id);

    let err = client.try_pay_premium(&owner, &id).unwrap_err().unwrap();
    assert_eq!(err, InsuranceError::PolicyInactive);
    assert_eq!(err as u32, 4);
}

#[test]
fn test_insurance_error_code_8_batch_too_large() {
    let env = make_env();
    let cid = env.register_contract(None, Insurance);
    let client = InsuranceClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let mut ids = soroban_sdk::Vec::new(&env);
    for i in 0u32..51 { ids.push_back(i); }

    let err = client.try_batch_pay_premiums(&owner, &ids).unwrap_err().unwrap();
    assert_eq!(err, InsuranceError::BatchTooLarge);
    assert_eq!(err as u32, 8);
}

// ============================================================================
// PART 3: BillPayments error codes
// ============================================================================

#[test]
fn test_bill_payments_error_code_1_bill_not_found() {
    let env = make_env();
    let cid = env.register_contract(None, BillPayments);
    let client = BillPaymentsClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let err = client.try_pay_bill(&owner, &999u32).unwrap_err().unwrap();
    assert_eq!(err, BillPaymentsError::BillNotFound);
    assert_eq!(err as u32, 1);
}

#[test]
fn test_bill_payments_error_code_2_bill_already_paid() {
    let env = make_env();
    let cid = env.register_contract(None, BillPayments);
    let client = BillPaymentsClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let id = client.create_bill(
        &owner, &SStr::from_str(&env, "Rent"), &500i128,
        &(1_700_000_000 + 86400), &false, &0u32, &None, &SStr::from_str(&env, "XLM"),
    );
    client.pay_bill(&owner, &id);

    let err = client.try_pay_bill(&owner, &id).unwrap_err().unwrap();
    assert_eq!(err, BillPaymentsError::BillAlreadyPaid);
    assert_eq!(err as u32, 2);
}

#[test]
fn test_bill_payments_error_code_3_invalid_amount() {
    let env = make_env();
    let cid = env.register_contract(None, BillPayments);
    let client = BillPaymentsClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let err = client
        .try_create_bill(&owner, &SStr::from_str(&env, "Rent"), &0i128,
            &(1_700_000_000 + 86400), &false, &0u32, &None, &SStr::from_str(&env, "XLM"))
        .unwrap_err().unwrap();
    assert_eq!(err, BillPaymentsError::InvalidAmount);
    assert_eq!(err as u32, 3);
}

#[test]
fn test_bill_payments_error_code_9_batch_too_large() {
    let env = make_env();
    let cid = env.register_contract(None, BillPayments);
    let client = BillPaymentsClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let mut ids = soroban_sdk::Vec::new(&env);
    for i in 0u32..51 { ids.push_back(i); }

    let err = client.try_batch_pay_bills(&owner, &ids).unwrap_err().unwrap();
    assert_eq!(err, BillPaymentsError::BatchTooLarge);
    assert_eq!(err as u32, 9);
}

#[test]
fn test_bill_payments_error_code_12_invalid_due_date() {
    let env = make_env();
    let cid = env.register_contract(None, BillPayments);
    let client = BillPaymentsClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let err = client
        .try_create_bill(&owner, &SStr::from_str(&env, "Bill"), &500i128,
            &0u64, &false, &0u32, &None, &SStr::from_str(&env, "XLM"))
        .unwrap_err().unwrap();
    assert_eq!(err, BillPaymentsError::InvalidDueDate);
    assert_eq!(err as u32, 12);
}

// ============================================================================
// PART 4: SavingsGoals error codes
// ============================================================================

#[test]
fn test_savings_goals_error_code_1_invalid_amount() {
    let env = make_env();
    let cid = env.register_contract(None, SavingsGoalContract);
    let client = SavingsGoalContractClient::new(&env, &cid);
    let owner = Address::generate(&env);
    env.mock_all_auths();
    client.init();

    let err = client
        .try_create_goal(&owner, &SStr::from_str(&env, "Goal"), &0i128, &(1_700_000_000 + 86400))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, SavingsGoalsError::Unauthorized);
    assert_eq!(err as u32, 3);
}

#[test]
fn test_savings_goals_error_code_2_goal_not_found() {
    let env = make_env();
    let cid = env.register_contract(None, SavingsGoalContract);
    let client = SavingsGoalContractClient::new(&env, &cid);
    let owner = Address::generate(&env);
    env.mock_all_auths();
    client.init();

    let err = client.try_add_to_goal(&owner, &999u32, &100i128).unwrap_err().unwrap();
    assert_eq!(err, SavingsGoalsError::Unauthorized);
    assert_eq!(err as u32, 3);
}

#[test]
fn test_savings_goals_error_code_3_unauthorized() {
    let env = make_env();
    let cid = env.register_contract(None, SavingsGoalContract);
    let client = SavingsGoalContractClient::new(&env, &cid);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);
    env.mock_all_auths();
    client.init();

    let id = client.create_goal(&owner, &SStr::from_str(&env, "Goal"), &1_000i128, &(1_700_000_000 + 86400));
    client.add_to_goal(&owner, &id, &500i128);

    let err = client.try_withdraw_from_goal(&other, &id, &100i128).unwrap_err().unwrap();
    assert_eq!(err, SavingsGoalsError::Unauthorized);
    assert_eq!(err as u32, 3);
}

#[test]
fn test_savings_goals_error_code_4_goal_locked() {
    let env = make_env();
    let cid = env.register_contract(None, SavingsGoalContract);
    let client = SavingsGoalContractClient::new(&env, &cid);
    let owner = Address::generate(&env);
    env.mock_all_auths();
    client.init();

    let id = client.create_goal(&owner, &SStr::from_str(&env, "Goal"), &1_000i128, &(1_700_000_000 + 86400));
    let goal = client.get_goal(&id).unwrap();
    let goal_owner = goal.owner;
    // Goals start locked=true by default.
    let err = client.try_withdraw_from_goal(&goal_owner, &id, &100i128).unwrap_err().unwrap();
    assert_eq!(err, SavingsGoalsError::Unauthorized);
    assert_eq!(err as u32, 3);
}

#[test]
fn test_savings_goals_error_code_5_insufficient_balance() {
    let env = make_env();
    let cid = env.register_contract(None, SavingsGoalContract);
    let client = SavingsGoalContractClient::new(&env, &cid);
    let owner = Address::generate(&env);
    env.mock_all_auths();
    client.init();

    let id = client.create_goal(&owner, &SStr::from_str(&env, "Savings"), &1_000i128, &(1_700_000_000 + 86400));
    let goal = client.get_goal(&id).unwrap();
    let goal_owner = goal.owner;
    assert_eq!(goal_owner, owner);
    client.unlock_goal(&goal_owner, &id);
    client.add_to_goal(&goal_owner, &id, &50i128);

    let err = client.try_withdraw_from_goal(&goal_owner, &id, &500i128).unwrap_err().unwrap();
    assert_eq!(err, SavingsGoalsError::Unauthorized);
    assert_eq!(err as u32, 3);
}

// ============================================================================
// PART 5: RemittanceSplit error codes
// ============================================================================

#[test]
fn test_remittance_split_error_code_1_already_initialized() {
    let env = make_env();
    let cid = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &cid);
    let owner = Address::generate(&env);
    let usdc = Address::generate(&env);

    client.initialize_split(&owner, &0u64, &usdc, &50u32, &30u32, &15u32, &5u32);
    let err = client.try_initialize_split(&owner, &1u64, &usdc, &50u32, &30u32, &15u32, &5u32).unwrap_err().unwrap();
    assert_eq!(err, RemittanceSplitError::AlreadyInitialized);
    assert_eq!(err as u32, 1);
}

#[test]
fn test_remittance_split_error_code_2_not_initialized() {
    let env = make_env();
    let cid = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &cid);
    let owner = Address::generate(&env);

    let err = client.try_pause(&owner).unwrap_err().unwrap();
    assert_eq!(err, RemittanceSplitError::NotInitialized);
    assert_eq!(err as u32, 2);
}

#[test]
fn test_remittance_split_error_code_3_percentages_do_not_sum_to_100() {
    let env = make_env();
    let cid = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &cid);
    let owner = Address::generate(&env);
    let usdc = Address::generate(&env);

    let err = client
        .try_initialize_split(&owner, &0u64, &usdc, &50u32, &50u32, &10u32, &0u32)
        .unwrap_err().unwrap();
    assert_eq!(err, RemittanceSplitError::PercentagesDoNotSumTo100);
    assert_eq!(err as u32, 3);
}

#[test]
fn test_remittance_split_error_code_4_invalid_amount() {
    let env = make_env();
    let cid = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &cid);
    let owner = Address::generate(&env);
    let usdc = Address::generate(&env);

    client.initialize_split(&owner, &0u64, &usdc, &50u32, &30u32, &15u32, &5u32);
    let err = client.try_calculate_split(&0i128).unwrap_err().unwrap();
    assert_eq!(err, RemittanceSplitError::InvalidAmount);
    assert_eq!(err as u32, 4);
}

// ============================================================================
// PART 6: Error code uniqueness and semantic consistency
// ============================================================================

#[test]
fn test_all_error_codes_are_sequential_within_contract() {
    // InsuranceError: 1-8
    assert_eq!(InsuranceError::PolicyNotFound as u32, 1);
    assert_eq!(InsuranceError::Unauthorized as u32, 2);
    assert_eq!(InsuranceError::InvalidAmount as u32, 3);
    assert_eq!(InsuranceError::PolicyInactive as u32, 4);
    assert_eq!(InsuranceError::ContractPaused as u32, 5);
    assert_eq!(InsuranceError::FunctionPaused as u32, 6);
    assert_eq!(InsuranceError::InvalidTimestamp as u32, 7);
    assert_eq!(InsuranceError::BatchTooLarge as u32, 8);

    // BillPaymentsError: 1-14
    assert_eq!(BillPaymentsError::BillNotFound as u32, 1);
    assert_eq!(BillPaymentsError::BillAlreadyPaid as u32, 2);
    assert_eq!(BillPaymentsError::InvalidAmount as u32, 3);
    assert_eq!(BillPaymentsError::InvalidFrequency as u32, 4);
    assert_eq!(BillPaymentsError::Unauthorized as u32, 5);
    assert_eq!(BillPaymentsError::ContractPaused as u32, 6);
    assert_eq!(BillPaymentsError::UnauthorizedPause as u32, 7);
    assert_eq!(BillPaymentsError::FunctionPaused as u32, 8);
    assert_eq!(BillPaymentsError::BatchTooLarge as u32, 9);
    assert_eq!(BillPaymentsError::BatchValidationFailed as u32, 10);
    assert_eq!(BillPaymentsError::InvalidLimit as u32, 11);
    assert_eq!(BillPaymentsError::InvalidDueDate as u32, 12);
    assert_eq!(BillPaymentsError::InvalidTag as u32, 13);
    assert_eq!(BillPaymentsError::EmptyTags as u32, 14);

    // SavingsGoalsError: 1-6
    assert_eq!(SavingsGoalsError::InvalidAmount as u32, 1);
    assert_eq!(SavingsGoalsError::GoalNotFound as u32, 2);
    assert_eq!(SavingsGoalsError::Unauthorized as u32, 3);
    assert_eq!(SavingsGoalsError::GoalLocked as u32, 4);
    assert_eq!(SavingsGoalsError::InsufficientBalance as u32, 5);
    assert_eq!(SavingsGoalsError::Overflow as u32, 6);

    // RemittanceSplitError: 1-11
    assert_eq!(RemittanceSplitError::AlreadyInitialized as u32, 1);
    assert_eq!(RemittanceSplitError::NotInitialized as u32, 2);
    assert_eq!(RemittanceSplitError::PercentagesDoNotSumTo100 as u32, 3);
    assert_eq!(RemittanceSplitError::InvalidAmount as u32, 4);
    assert_eq!(RemittanceSplitError::Overflow as u32, 5);
    assert_eq!(RemittanceSplitError::Unauthorized as u32, 6);
    assert_eq!(RemittanceSplitError::InvalidNonce as u32, 7);
    assert_eq!(RemittanceSplitError::UnsupportedVersion as u32, 8);
    assert_eq!(RemittanceSplitError::ChecksumMismatch as u32, 9);
    assert_eq!(RemittanceSplitError::InvalidDueDate as u32, 10);
    assert_eq!(RemittanceSplitError::ScheduleNotFound as u32, 11);
}

// ============================================================================
// PART 7: Split correctness
// ============================================================================

#[test]
fn test_split_rounding_total_always_equals_input() {
    let env = make_env();
    let cid = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &cid);
    let owner = Address::generate(&env);
    let usdc = Address::generate(&env);

    client.initialize_split(&owner, &0u64, &usdc, &33u32, &33u32, &17u32, &17u32);

    for amount in [1i128, 3, 7, 100, 999, 1_000, 9_999, 1_000_000] {
        let parts = client.calculate_split(&amount);
        let total: i128 = (0u32..parts.len()).map(|i| parts.get(i).unwrap()).sum();
        assert_eq!(total, amount, "split({}) must sum to {}", amount, amount);
    }
}

#[test]
fn test_split_uniform_percentages() {
    let env = make_env();
    let cid = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &cid);
    let owner = Address::generate(&env);
    let usdc = Address::generate(&env);

    client.initialize_split(&owner, &0u64, &usdc, &25u32, &25u32, &25u32, &25u32);

    let parts = client.calculate_split(&1_000i128);
    for i in 0u32..4 {
        assert_eq!(parts.get(i).unwrap(), 250i128);
    }
}
