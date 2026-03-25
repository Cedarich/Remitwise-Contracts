#![no_std]
use soroban_sdk::{symbol_short, Address, BytesN, Env, IntoVal, Symbol};

fn get_killswitch_id(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&symbol_short!("K_ID"))
        .unwrap()
}

pub fn pay_premium(env: Env, _policy_id: BytesN<32>) {
    let killswitch_id = get_killswitch_id(&env);
    let is_paused: bool = env.invoke_contract(
        &killswitch_id,
        &symbol_short!("is_paused"),
        soroban_sdk::vec![&env, Symbol::new(&env, "insurance").into_val(&env)],
    );
    
    if is_paused {
        panic!("Contract is currently paused for emergency maintenance.");
    }
    // ... rest of the logic
}

