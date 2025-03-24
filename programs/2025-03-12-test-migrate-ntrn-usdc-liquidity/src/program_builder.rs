use std::str::FromStr;

use cosmwasm_std::{to_json_binary, Decimal};
use deployer_lib::EMPTY_VEC;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_library_utils::LibraryAccountType;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};

/// Write your program using the program builder
pub fn program_builder(params: deployer_lib::ProgramParams) -> ProgramConfig {
    // program params
    let owner = params.get("owner");
    let init_lp_token_denom = params.get("init_lp_token_denom");
    let usdc_dntrn_lp_token_denom = params.get("usdc_dntrn_lp_token_denom");
    let usdc_ntrn_pool_addr = params.get("usdc_ntrn_pool_addr");
    let usdc_dntrn_pool_addr = params.get("usdc_dntrn_pool_addr");
    let ntrn_denom = params.get("ntrn_denom");
    let dntrn_denom = params.get("dntrn_denom");
    let usdc_denom = params.get("usdc_denom");
    let drop_liquid_staker_addr = params.get("drop_liquid_staker_addr");
    let pool_max_spread = params.get("pool_max_spread");
    let neutron_dao_addr = params.get("neutron_dao_addr");
    let security_dao_addr = params.get("security_dao_addr");
    let double_sided_min = params.get("double_sided_min");
    let double_sided_max = params.get("double_sided_max");
    let authorizations_allowed_list = params.get_array("authorizations_allowed_list");

    let permissioned_all_mode =
        valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
            valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                authorizations_allowed_list,
            ),
        );

    // Domains
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm("neutron".to_string());

    let mut builder = ProgramConfigBuilder::new("test-migrate-ntrn", &owner);

    let acc_lp_receiver = builder.add_account(AccountInfo::new(
        "lp_receiver_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_lp_withdraw = builder.add_account(AccountInfo::new(
        "lp_withdraw_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_withdrawn_liquidity = builder.add_account(AccountInfo::new(
        "withdrawn_liquidity_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_ready_to_stake = builder.add_account(AccountInfo::new(
        "ready_to_stake_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_ready_to_lp = builder.add_account(AccountInfo::new(
        "ready_to_lp_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_holder = builder.add_account(AccountInfo::new(
        "holder_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_unlock = builder.add_account(AccountInfo::new(
        "unlock_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Libraries

    // lp forwarder
    let lp_token_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_lp_receiver.clone(),
        output_addr: acc_lp_withdraw.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(init_lp_token_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let lib_lp_token_forwarder = builder.add_library(LibraryInfo::new(
        "lp_token_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(lp_token_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_lp_token_forwarder,
        vec![&acc_lp_receiver],
        vec![&acc_lp_withdraw],
    );

    // usdc - ntrn astroport withdrawer
    let usdc_ntrn_withdrawer_config = valence_astroport_withdrawer::msg::LibraryConfig {
        input_addr: acc_lp_withdraw.clone(),
        output_addr: acc_withdrawn_liquidity.clone(),
        pool_addr: usdc_ntrn_pool_addr,
        withdrawer_config: valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
            asset_data: valence_library_utils::liquidity_utils::AssetData {
                asset1: usdc_denom.clone(),
                asset2: ntrn_denom.clone(),
            },
        },
    };

    let lib_usdc_ntrn_withdrawer = builder.add_library(LibraryInfo::new(
        "usdc_ntrn_withdrawer".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportWithdrawer(usdc_ntrn_withdrawer_config.clone()),
    ));

    builder.add_link(
        &lib_usdc_ntrn_withdrawer,
        vec![&acc_lp_withdraw],
        vec![&acc_withdrawn_liquidity],
    );

    // ntrn to staker forwarder
    let ntrn_to_staker_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_withdrawn_liquidity.clone(),
        output_addr: acc_ready_to_stake.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let lib_ntrn_to_staker_forwarder = builder.add_library(LibraryInfo::new(
        "ntrn_to_staker_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(ntrn_to_staker_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_ntrn_to_staker_forwarder,
        vec![&acc_withdrawn_liquidity],
        vec![&acc_ready_to_stake],
    );

    // usdc to ready to lp forwarder
    let usdc_to_ready_to_lp_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_withdrawn_liquidity.clone(),
        output_addr: acc_ready_to_lp.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(usdc_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let lib_usdc_to_ready_to_lp_forwarder = builder.add_library(LibraryInfo::new(
        "usdc_to_ready_to_lp_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(usdc_to_ready_to_lp_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_usdc_to_ready_to_lp_forwarder,
        vec![&acc_withdrawn_liquidity],
        vec![&acc_ready_to_lp],
    );

    // Drop liquid staker lib
    let drop_liquid_staker_config = valence_drop_liquid_staker::msg::LibraryConfig {
        input_addr: acc_ready_to_stake.clone(),
        output_addr: acc_ready_to_lp.clone(),
        liquid_staker_addr: drop_liquid_staker_addr,
        denom: ntrn_denom,
    };

    let lib_drop_liquid_staker = builder.add_library(LibraryInfo::new(
        "drop_liquid_staker".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceDropLiquidStaker(drop_liquid_staker_config.clone()),
    ));

    builder.add_link(
        &lib_drop_liquid_staker,
        vec![&acc_ready_to_stake],
        vec![&acc_ready_to_lp],
    );

    // astroport lper library
    let max_spread = if pool_max_spread.is_empty() {
        None
    } else {
        Some(
            Decimal::from_str(pool_max_spread.as_str())
                .expect("pool_max_spread must be valid Decimal string"),
        )
    };
    let astroport_lper_config = valence_astroport_lper::msg::LibraryConfig {
        input_addr: acc_ready_to_lp.clone(),
        output_addr: acc_holder.clone(),
        pool_addr: usdc_dntrn_pool_addr.clone(),
        lp_config: valence_astroport_lper::msg::LiquidityProviderConfig {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
            asset_data: valence_library_utils::liquidity_utils::AssetData {
                asset1: usdc_denom.clone(),
                asset2: dntrn_denom.clone(),
            },
            max_spread,
        },
    };

    let lib_astroport_lper = builder.add_library(LibraryInfo::new(
        "astroport_lper".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportLper(astroport_lper_config.clone()),
    ));

    builder.add_link(
        &lib_astroport_lper,
        vec![&acc_ready_to_lp],
        vec![&acc_holder],
    );

    // lp to dao forwarder
    let lp_to_dao_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_holder.clone(),
        output_addr: valence_library_utils::LibraryAccountType::Addr(neutron_dao_addr.clone()),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(usdc_dntrn_lp_token_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let lib_lp_to_dao_forwarder = builder.add_library(LibraryInfo::new(
        "lp_to_dao_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(lp_to_dao_forwarder_config.clone()),
    ));

    builder.add_link(&lib_lp_to_dao_forwarder, vec![&acc_holder], EMPTY_VEC);

    // lp to ready to unlock forwarder
    let lp_to_unlock_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_holder.clone(),
        output_addr: acc_unlock.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(usdc_dntrn_lp_token_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let lib_lp_to_unlock_forwarder = builder.add_library(LibraryInfo::new(
        "lp_to_unlock_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(lp_to_unlock_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_lp_to_unlock_forwarder,
        vec![&acc_holder],
        vec![&acc_unlock],
    );

    // usdc - dntrn astroport withdrawer
    let usdc_dntrn_withdrawer_config = valence_astroport_withdrawer::msg::LibraryConfig {
        input_addr: acc_unlock.clone(),
        output_addr: LibraryAccountType::Addr(neutron_dao_addr.clone()),
        pool_addr: usdc_dntrn_pool_addr,
        withdrawer_config: valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
            asset_data: valence_library_utils::liquidity_utils::AssetData {
                asset1: usdc_denom.clone(),
                asset2: dntrn_denom.clone(),
            },
        },
    };

    let lib_usdc_dntrn_withdrawer = builder.add_library(LibraryInfo::new(
        "usdc_dntrn_withdrawer".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportWithdrawer(usdc_dntrn_withdrawer_config.clone()),
    ));

    builder.add_link(&lib_usdc_dntrn_withdrawer, vec![&acc_unlock], EMPTY_VEC);

    // Authorizations

    // forward usdc/ntrn lp tokens
    let forward_lp_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_lp_token_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "forward".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(forward_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(permissioned_all_mode.clone())
        .with_label("forward_lp")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // withdraw usdc/ntrn
    let withdraw_usdc_ntrn_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_usdc_ntrn_withdrawer.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "withdraw_liquidity".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(withdraw_usdc_ntrn_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(permissioned_all_mode.clone())
        .with_label("withdraw_usdc_ntrn")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // forward usdc to ready to lp
    let forward_usdc_ready_to_lp_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_usdc_to_ready_to_lp_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    // "forward".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(forward_usdc_ready_to_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(permissioned_all_mode.clone())
        .with_label("forward_usdc_ready_to_lp")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // forward usdc to ready to lp
    let forward_ntrn_to_staker_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_to_staker_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "forward".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(forward_ntrn_to_staker_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(permissioned_all_mode.clone())
        .with_label("forward_ntrn_to_staker")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Liquid stake function
    let liquid_stake_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_drop_liquid_staker.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "liquid_stake".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(liquid_stake_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("liquid_stake")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Provide double sided liquidity
    let expected_pool_ratio_range =
        Some(valence_library_utils::liquidity_utils::DecimalRange::new(
            Decimal::from_str(double_sided_min.as_str())
                .expect("double_sided_min must be parsed into Decimal"),
            Decimal::from_str(double_sided_max.as_str())
                .expect("double_sided_max must be parsed into Decimal"),
        ));
    let double_sided_lp_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_astroport_lper.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeValue(
                    vec![
                        "process_function".to_string(),
                        "provide_double_sided_liquidity".to_string(),
                        "expected_pool_ratio_range".to_string(),
                    ],
                    to_json_binary(&expected_pool_ratio_range)
                        .expect("expected_pool_ratio_range must parse to binary"),
                )]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(double_sided_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("double_sided_lp")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Provide double sided liquidity by DAO
    let double_sided_lp_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_astroport_lper.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "provide_double_sided_liquidity".to_string(),
                    "expected_pool_ratio_range".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(double_sided_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![security_dao_addr.clone()],
                ),
            ),
        )
        .with_label("double_sided_lp_sec_dao")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Provide single sided liquidity by DAO
    let single_sided_lp_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_astroport_lper.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "provide_single_sided_liquidity".to_string(),
                    "asset".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(single_sided_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![security_dao_addr.clone()],
                ),
            ),
        )
        .with_label("single_sided_lp_sec_dao")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Forward usdc dntrn lp tokens to neutron DAO
    let forward_lp_to_dao_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_lp_to_dao_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "forward".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(forward_lp_to_dao_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone()],
                ),
            ),
        )
        .with_label("forward_lp_to_dao")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Forward lp tokens to unlock
    let update_lp_forwarder_config_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_lp_to_unlock_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "update_config".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "update_config".to_string(),
                    "new_config".to_string(),
                ])]),
            },
        })
        .build();
    let forward_lp_to_unlock_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_lp_to_unlock_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "forward".to_string(),
                ])]),
            },
        })
        .build();
    let withdraw_from_astro_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_usdc_dntrn_withdrawer.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "withdraw_liquidity".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(update_lp_forwarder_config_func)
        .with_function(forward_lp_to_unlock_func)
        .with_function(withdraw_from_astro_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_label("withdraw_lp_token")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update usdc/ntrn lp tokens forward config
    let update_lp_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_lp_token_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "update_config".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "update_config".to_string(),
                    "new_config".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(update_lp_forward_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_label("update_lp_forward_config")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    builder.build()
}
