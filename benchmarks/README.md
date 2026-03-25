# Savings Goals Gas Benchmarks

## Overview
This document tracks the CPU and Memory resource consumption (gas benchmarking) for heavy operational paths in the `savings_goals` Soroban smart contract.

## Test Instructions
To reproduce these benchmark numbers locally, run the following command from the repository root:
```bash
RUST_TEST_THREADS=1 cargo test -p savings_goals --test gas_bench -- --nocapture
```

## Security Assumptions
- **Execution Scaling Constraints**: Batch loops (`batch_add_to_goals`, `execute_due_savings_schedules`) constrain execution via maximum capacities (e.g., `MAX_BATCH_SIZE: u32 = 50`) ensuring transactions cannot exceed Soroban limits or induce Out of Gas errors.
- **State Integrity**: No external untrusted contract interfaces are invoked during localized mass schedule executions, removing re-entrancy attack surfaces.
- **Edge Cases**: `batch_add_to_goals` explicitly validates goals are owned by the `caller`. If a user injects an external goal ID via UI parameters, the batch transaction will hard panic and roll back with `Not owner of all goals`. `execute_due_savings_schedules` processes independent of caller allowing cron execution without unauthorized balance manipulation.

## Baseline Metrics & Regression Thresholds
_Warning: If any future optimization exceeds these `CPU` or `Memory` ceilings by > 10%, a thorough regression review natively against `env.budget()` is mandatory._

| Operation | Scenario | CPU Instructions | Memory Allocated | Status |
| :--- | :--- | :---: | :---: | :---: |
| `get_all_goals` | 100 goals | 2,976,312 | 540,833 | ✅ Verified |
| `batch_add_to_goals` | 50 items | 3,037,951 | 615,851 | ✅ Verified |
| `execute_due_savings_schedules` | 50 schedules | 3,178,232 | 701,755 | ✅ Verified |
| `create_savings_schedule` | Single schedule | 106,701 | 19,795 | ✅ Verified |

*Metrics extracted from local env budget execution tracker using host trace APIs.*
