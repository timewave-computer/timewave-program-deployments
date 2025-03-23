# Neutron dICS Programs

There are five programs Timewave is deploying to support Neutron's monetary policy as they become a sovereign Proof of Stake chain.
These include the following.

1. **NTRN Allocation Program**: This program receives 250M NTRN and allocates them to the other programs. See [program details](programs/2025-03-23-prod-dICS-ntrn-allocation/README.md).
2. **Instant liquid-stake NTRN Program**: This program receives 125M NTRN which is liquid staked using the Drop protocol immediately. 100M dNTRN is returned to the DAO. 25M dNTRN is sent to bootstrap NTRN and dNTRN liquidity, i.e., program 4. See [program details](programs/2025-03-23-prod-dICS-ntrn-instant-ls/README.md).
3. **Gradual liquid-stake NTRN program**: This program receives 100M NTRN and it liquid stakes these gradually over three months. See [program details](programs/2025-03-23-prod-dICS-gradual-ls/README.md).
4. **Bootstrap NTRN and dNTRN liquidity program**: This program receives NTRN from program 1 and dNTRN from program 2. It provides liquidity to the Astroport pool and returns the LP share tokens to the Neutron DAO.
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