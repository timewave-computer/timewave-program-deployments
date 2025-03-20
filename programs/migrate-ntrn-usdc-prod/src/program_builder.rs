use std::str::FromStr;

use cosmwasm_std::{to_json_binary, Decimal};
use deployer_lib::EMPTY_VEC;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
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
    let usdc_ntrn_lp_denom = params.get("usdc_ntrn_lp_denom");
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
    let usdc_ntrn_lp_forwarder_amount = params.get("usdc_ntrn_lp_forwarder_amount");
    let usdc_ntrn_lp_forwarder_time_constrain = params.get("usdc_ntrn_lp_forwarder_time_constrain");
    let usdc_forwarder_amount = params.get("usdc_forwarder_amount");
    let usdc_forwarder_time_constrain = params.get("usdc_forwarder_time_constrain");
    let ntrn_forwarder_amount = params.get("ntrn_forwarder_amount");
    let ntrn_forwarder_time_constrain = params.get("ntrn_forwarder_time_constrain");
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

    let mut builder =
        ProgramConfigBuilder::new("Migrate NTRN/USDC to dNTRN/USDC Production V1", &owner);

    let acc_ntrn_usdc_lp_receiver = builder.add_account(AccountInfo::new(
        "ntrn_usdc_lp_receiver".to_string(),
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

    // Libraries
    // lp forwarder
    let usdc_ntrn_lp_forwarder_constraints = if usdc_ntrn_lp_forwarder_time_constrain.is_empty()
        || usdc_ntrn_lp_forwarder_time_constrain == "0"
    {
        None
    } else {
        Some(cw_utils::Duration::Time(
            usdc_ntrn_lp_forwarder_time_constrain
                .parse()
                .expect("usdc_ntrn_lp_forwarder_time_constrain is not a valid number"),
        ))
    };
    let ntrn_usdc_lp_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_ntrn_usdc_lp_receiver.clone(),
        output_addr: acc_lp_withdraw.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(usdc_ntrn_lp_denom.clone()),
            usdc_ntrn_lp_forwarder_amount
                .parse()
                .expect("usdc_ntrn_lp_forwarder_amount is not a valid number"),
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(
            usdc_ntrn_lp_forwarder_constraints,
        ),
    };

    let lib_ntrn_usdc_lp_forwarder = builder.add_library(LibraryInfo::new(
        "usdc_ntrn_lp_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(ntrn_usdc_lp_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_ntrn_usdc_lp_forwarder,
        vec![&acc_ntrn_usdc_lp_receiver],
        vec![&acc_lp_withdraw],
    );

    // usdc - ntrn astroport withdrawer
    let ntrn_usdc_withdrawer_config = valence_astroport_withdrawer::msg::LibraryConfig {
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

    let lib_ntrn_usdc_withdrawer = builder.add_library(LibraryInfo::new(
        "ntrn_usdc_withdrawer".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportWithdrawer(ntrn_usdc_withdrawer_config.clone()),
    ));

    builder.add_link(
        &lib_ntrn_usdc_withdrawer,
        vec![&acc_lp_withdraw],
        vec![&acc_withdrawn_liquidity],
    );

    // usdc to ready to lp forwarder
    let usdc_forwarder_constraints =
        if usdc_forwarder_time_constrain.is_empty() || usdc_forwarder_time_constrain == "0" {
            None
        } else {
            Some(cw_utils::Duration::Time(
                usdc_forwarder_time_constrain
                    .parse()
                    .expect("usdc_forwarder_time_constrain is not a valid number"),
            ))
        };
    let usdc_to_ready_to_lp_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_withdrawn_liquidity.clone(),
        output_addr: acc_ready_to_lp.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(usdc_denom.clone()),
            usdc_forwarder_amount
                .parse()
                .expect("usdc_forwarder_amount is not a valid number"),
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(
            usdc_forwarder_constraints,
        ),
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

    // ntrn to staker forwarder
    let ntrn_forwarder_constraints =
        if ntrn_forwarder_time_constrain.is_empty() || ntrn_forwarder_time_constrain == "0" {
            None
        } else {
            Some(cw_utils::Duration::Time(
                ntrn_forwarder_time_constrain
                    .parse()
                    .expect("ntrn_forwarder_time_constrain is not a valid number"),
            ))
        };
    let ntrn_to_staker_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_withdrawn_liquidity.clone(),
        output_addr: acc_ready_to_stake.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
            ntrn_forwarder_amount
                .parse()
                .expect("ntrn_forwarder_amount is not a valid number"),
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(
            ntrn_forwarder_constraints,
        ),
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
        output_addr: valence_library_utils::LibraryAccountType::Addr(neutron_dao_addr.clone()),
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

    builder.add_link(&lib_astroport_lper, vec![&acc_ready_to_lp], EMPTY_VEC);

    // Authorizations
    // forward usdc/ntrn lp tokens
    let forward_lp_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_usdc_lp_forwarder.clone())
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
        .with_label("forward_lp")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // withdraw ntrn/usdc
    let withdraw_usdc_ntrn_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_usdc_withdrawer.clone())
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
        .with_label("withdraw_usdc_ntrn")
        .with_mode(permissioned_all_mode.clone())
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
                    "forward".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(forward_usdc_ready_to_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("forward_usdc_ready_to_lp")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // forward ntrn to staker
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
        .with_label("forward_ntrn_to_staker")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Liquid stake ntrn function
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
        .with_label("liquid_stake_ntrn")
        .with_mode(permissioned_all_mode.clone())
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
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Provide double sided liquidity by DAO
    let secure_double_sided_lp_func = AtomicFunctionBuilder::new()
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
        .with_function(secure_double_sided_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_double_sided_lp_sec_dao")
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![security_dao_addr.clone()],
                ),
            ),
        )
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Provide single sided liquidity by DAO
    let secure_single_sided_lp_func = AtomicFunctionBuilder::new()
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
        .with_function(secure_single_sided_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_single_sided_lp_sec_dao")
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![security_dao_addr.clone()],
                ),
            ),
        )
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update ntrn/usdc lp tokens forward config
    let secure_update_lp_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_usdc_lp_forwarder.clone())
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
        .with_function(secure_update_lp_forward_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_update_lp_forward_config")
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update ntrn forwarder config
    let secure_update_ntrn_forwarder_config_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_to_staker_forwarder.clone())
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
        .with_function(secure_update_ntrn_forwarder_config_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_update_ntrn_forwarder_config")
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update usdc forwarder config
    let secure_update_usdc_forwarder_config_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_usdc_to_ready_to_lp_forwarder.clone())
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
        .with_function(secure_update_usdc_forwarder_config_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_update_usdc_forwarder_config")
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    builder.build()
}
