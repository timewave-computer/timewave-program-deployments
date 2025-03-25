# Neutron dICS Programs

There are five programs Timewave is deploying to support Neutron's monetary policy as they become a sovereign Proof of Stake chain.
These include the following.

1. **NTRN Allocation Program**: This program receives 250M NTRN and allocates them to the other programs. See [program details](programs/2025-03-23-prod-dICS-ntrn-allocation/README.md).
2. **Instant liquid-stake NTRN Program**: This program receives 125M NTRN which is liquid staked using the Drop protocol immediately. 100M dNTRN is returned to the DAO. 25M dNTRN is sent to bootstrap NTRN and dNTRN liquidity, i.e., program 4. See [program details](programs/2025-03-23-prod-dICS-ntrn-instant-ls/README.md).
3. **Gradual liquid-stake NTRN program**: This program receives 100M NTRN and it liquid stakes these gradually over three months. See [program details](programs/2025-03-23-prod-dICS-gradual-ls/README.md).
4. **Bootstrap NTRN and dNTRN liquidity program**: This program receives NTRN from program 1 and dNTRN from program 2. It provides liquidity to the Astroport pool and returns the LP share tokens to the Neutron DAO. See [program details](programs/2025-03-23-prod-bootstrap-ntrn-dntrn-liquidity/README.md).
5. **Migrate USDC-NTRN liquidity program**: This program receives the USDC-NTRN liquidity LP tokens from the Neutron DAO. It withdraws the liquidity, liquid stakes the NTRN, and enters the USDC-dNTRN liquidity pool. Liquidity tokens are sent back to the Neutron DAO. See [program details](programs/2025-03-23-prod-migrate-usdc-ntrn-liquidity/README.md).

```mermaid
graph TD;
    %% Define subgraphs for better organization
    subgraph Input
        N1((Neutron DAO))
    end

    subgraph Programs
        P1[[Program 1<br>NTRN Allocation]]
        P2[[Program 2<br>Instant liquid stake]]
        P3[[Program 3<br>Gradual liquid stake]]
        P4[[Program 4<br>Bootstrap NTRN-dNTRN]]
        P5[[Program 5<br>Migrate USDC-NTRN]]
    end

    subgraph Output
        N2((Neutron DAO))
    end

    %% Main NTRN flow
    N1 --250M NTRN--> P1
    P1 --125M NTRN--> P2
    P1 --100M NTRN--> P3
    P1 --25M NTRN--> P4

    %% Liquid staking flows
    P2 --100M dNTRN--> N2
    P2 --25M dNTRN--> P4
    P3 --100M dNTRN--> N2

    %% Liquidity flows
    P4 --NTRN-dNTRN-LP shares--> N2
    N1 --USDC-NTRN-LP shares--> P5
    P5 --USDC-dNTRN-LP shares--> N2

    %% Layout improvements
    classDef program fill:#f9f,stroke:#333,stroke-width:2px;
    classDef dao fill:#bbf,stroke:#333,stroke-width:2px;
    class P1,P2,P3,P4,P5 program;
    class N1,N2 dao;
```

## Security Model

Each program implements a two-tier security model:

1. **Low Security Operations**
   - Authorized by addresses in the `operator_list`
   - Includes routine operations like:
     - Splitting NTRN
     - Liquid staking NTRN
     - Providing liquidity
     - Withdrawing liquidity
   - No call limits on authorized functions

2. **High Security Operations**
   - Requires authorization from either the Neutron DAO or the Security DAO
   - Includes critical operations like:
     - Updating program configurations
     - Returning unspent tokens
     - Modifying security parameters
   - No call limits on authorized functions

Each program maintains its own set of authorized operators and security parameters, but follows this consistent security model across all programs in the dICS initiative.

## Program Deployment Order

Due to dependencies in the programs, programs should be deployed in reverse order. The `program_param` TOML files will need to be updated with addresses from the programs.
- Program 5 is independent of programs 1-4
- Program 2 needs the receiver address for Program 4
- Program 1 needs receiver addresses for Programs 1, 2, and 3

## Testing and Rehearsals
Before using in production, please do the following:
1. Make a copy of `mainnet.toml` files in each program's `program_param` folder and give it a suitable name. A good name is `<test_date>_<chain_name>_<label>.toml`.
2. Ensure every subroutine from every program has been executed in the tests. Use the Subroutine Authorization Matrix to confirm test results.

Mainnet fork deployment checklist
1
2
3. Deployed. Program ID 10
4. Deployed. Program ID 8
5. Deployed. Program ID 7

### Subroutine Authorization Matrix

| Program | Subroutine | Authorization | Parameter Restrictions | Test Status |
|---------|------------|---------------|------------------------|-------------|
| **Program 1: NTRN Allocation** |
| | `split_ntrn` | Operators | Must include "process_function" and "liquid_stake" parameters | |
| | `update_split_config` | Neutron DAO + Security DAO | Must include "update_config" and "new_config" parameters | |
| **Program 2: Instant Liquid Stake** |
| | `liquid_stake` | Operators | Must include "process_function" and "liquid_stake" parameters | |
| | `split_to_provide` | Operators | Must include "process_function" and "split" parameters | |
| | `secure_update_split_config` | Neutron DAO + Security DAO | Must include "update_config" and "new_config" parameters | |
| **Program 3: Gradual Liquid Stake** |
| | `liquid_stake_batch` | Operators | Must include "process_function" and "liquid_stake" parameters | |
| | `forward_batch` | Operators | Must include "process_function" and "forward" parameters | |
| | `secure_update_forwarder_config` | Neutron DAO + Security DAO | Must include "update_config" and "new_config" parameters | |
| **Program 4: Bootstrap NTRN-dNTRN** |
| | `secure_send_tokens_to_dao` | Security DAO | Controlled by forwarder config parameters | |
| | `double_sided_lp` | Operators | Pool ratio must be between constrained `expected_pool_ratio_min` and `expected_pool_ratio_max` | |
| | `secure_double_sided_lp` | Security DAO | Can set the `expected_pool_ratio_min` and `expected_pool_ratio_max` while providing liquidity | |
| | `secure_single_sided_lp` | Security DAO | Can set the `expected_pool_ratio_min` and `expected_pool_ratio_max` and `max_spread` | |
| | `secure_update_return_forwarder_config` | Neutron DAO + Security DAO | Must include "update_config" and "new_config" parameters | |
| **Program 5: Migrate USDC-NTRN** |
| | `forward_usdc_ntrn_lp_batch` | Operators | Must include "process_function" and "forward" parameters | |
| | `withdraw_usdc_ntrn_liquidity` | Operators | Must include "process_function" and "withdraw_liquidity" parameters | |
| | `forward_usdc_to_provide_ready_account` | Operators | Must include "process_function" and "forward" parameters | |
| | `liquid_stake_ntrn` | Operators | Must include "process_function" and "liquid_stake" parameters | |
| | `provide_double_sided_liquidity` | Operators | Pool ratio must be between `expected_pool_ratio_min` and `expected_pool_ratio_max` | |
| | `secure_provide_double_sided_liquidity` | Neutron DAO + Security DAO | Can set the expected pool ration with `expected_pool_ratio_min` and `expected_pool_ratio_max` | |
| | `secure_provide_single_sided_liquidity` | Neutron DAO + Security DAO | Must specify which asset to provide | |
| | `secure_return_unspent_tokens` | Neutron DAO + Security DAO | Must include "process_function" and "forward" parameters | |
| | `secure_update_lp_forward_config` | Neutron DAO + Security DAO | Must include "update_config" and "new_config" parameters | |
| | `secure_update_usdc_forwarder_config` | Neutron DAO + Security DAO | Must include "update_config" and "new_config" parameters | |