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
    // Get the token denom params
    let usdc_denom = params.get("usdc_denom");
    let ntrn_denom = params.get("ntrn_denom");
    let dntrn_denom = params.get("dntrn_denom");
    let usdc_ntrn_lp_denom = params.get("usdc_ntrn_lp_denom");
    // Get USDC-NTRN lp token batching params
    let usdc_ntrn_lp_max_batch_size = params.get("usdc_ntrn_lp_max_batch_size");
    let usdc_ntrn_lp_batch_interval_seconds = params.get("usdc_ntrn_lp_batch_interval_seconds");
    // Get USDC-NTRN pool params
    let usdc_ntrn_pool_addr = params.get("usdc_ntrn_pool_addr");
    // Get drop liquid staker params
    let drop_liquid_staker_addr = params.get("drop_liquid_staker_addr");
    // Get liquidity provisioning params
    let usdc_dntrn_pool_addr = params.get("usdc_dntrn_pool_addr");
    let expected_pool_ratio_min = params.get("expected_pool_ratio_min");
    let expected_pool_ratio_max = params.get("expected_pool_ratio_max");
    let pool_max_spread = params.get("pool_max_spread");
    // Get actor addresses
    let neutron_dao_addr = params.get("neutron_dao_addr");
    let security_dao_addr = params.get("security_dao_addr");
    let operator_list = params.get_array("operator_list");

    // Get forwarder params. These are the max amounts that can be forwarded in a single call.
    let usdc_forwarder_max_amount = params.get("usdc_forwarder_max_amount");
    let return_forwarder_max_amount = params.get("return_forwarder_max_amount");

    let permissioned_all_mode =
        valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
            valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                operator_list,
            ),
        );

    // Domains
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm("neutron".to_string());

    // Get a program config builder
    let mut builder = ProgramConfigBuilder::new(
        "Valence Program dICS 5: Migrate NTRN/USDC to dNTRN/USDC Production V1",
        &owner,
    );

    // Configure the accounts
    let acc_ntrn_usdc_lp_receiver = builder.add_account(AccountInfo::new(
        "ntrn_usdc_lp_receiver".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_withdraw_ready = builder.add_account(AccountInfo::new(
        "withdraw_ready_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_interim = builder.add_account(AccountInfo::new(
        "interim_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let acc_provide_ready = builder.add_account(AccountInfo::new(
        "provide_ready_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Libraries
    // Forwarder to batch and forward NTRN/USDC LP tokens
    let usdc_ntrn_lp_forwarder_constraints = if usdc_ntrn_lp_batch_interval_seconds.is_empty()
        || usdc_ntrn_lp_batch_interval_seconds == "0"
    {
        None
    } else {
        Some(cw_utils::Duration::Time(
            usdc_ntrn_lp_batch_interval_seconds
                .parse()
                .expect("usdc_ntrn_lp_batch_interval_seconds is not a valid number"),
        ))
    };
    let ntrn_usdc_lp_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_ntrn_usdc_lp_receiver.clone(),
        output_addr: acc_withdraw_ready.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(usdc_ntrn_lp_denom.clone()),
            usdc_ntrn_lp_max_batch_size
                .parse()
                .expect("usdc_ntrn_lp_max_batch_size is not a valid number"),
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
        vec![&acc_withdraw_ready],
    );

    // Library to withdraw NTRN/USDC LP tokens
    let ntrn_usdc_withdrawer_config = valence_astroport_withdrawer::msg::LibraryConfig {
        input_addr: acc_withdraw_ready.clone(),
        output_addr: acc_interim.clone(),
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
        vec![&acc_withdraw_ready],
        vec![&acc_interim],
    );

    // Library to forward USDC tokens between Interim and Provide Ready accounts
    let usdc_to_provide_ready_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_interim.clone(),
        output_addr: acc_provide_ready.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(usdc_denom.clone()),
            usdc_forwarder_max_amount
                .parse()
                .expect("usdc_forwarder_max_amount is not a valid number"),
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let lib_usdc_to_provide_ready_forwarder = builder.add_library(LibraryInfo::new(
        "usdc_to_provide_ready_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(usdc_to_provide_ready_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_usdc_to_provide_ready_forwarder,
        vec![&acc_interim],
        vec![&acc_provide_ready],
    );

    // Configure the Drop Liquid Staker library.
    // It will take NTRN tokens from the Interim account and liquid stake them.
    // It will send dNTRN to the Provide Ready account.
    let drop_liquid_staker_config = valence_drop_liquid_staker::msg::LibraryConfig {
        input_addr: acc_interim.clone(),
        output_addr: acc_provide_ready.clone(),
        liquid_staker_addr: drop_liquid_staker_addr.clone(),
        denom: ntrn_denom.clone(),
    };

    let lib_drop_liquid_staker = builder.add_library(LibraryInfo::new(
        "drop_liquid_staker".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceDropLiquidStaker(drop_liquid_staker_config.clone()),
    ));

    builder.add_link(
        &lib_drop_liquid_staker,
        vec![&acc_interim],
        vec![&acc_provide_ready],
    );

    // Configure the Astroport LPer library.
    // It will provide double sided liquidity to the dNTRN/USDC pool.
    // It will take USDC and dNTRN tokens from the Provide Ready account.
    // It will send the LP tokens to the Neutron DAO account.
    // Note: Asset order is USDC (asset1) and dNTRN (asset2) as per Astroport pool configuration
    let max_spread = if pool_max_spread.is_empty() {
        None
    } else {
        Some(
            Decimal::from_str(pool_max_spread.as_str())
                .expect("pool_max_spread must be valid Decimal string"),
        )
    };

    let astroport_lper_config = valence_astroport_lper::msg::LibraryConfig {
        input_addr: acc_provide_ready.clone(),
        output_addr: valence_library_utils::LibraryAccountType::Addr(neutron_dao_addr.clone()),
        pool_addr: usdc_dntrn_pool_addr.clone(),
        lp_config: valence_astroport_lper::msg::LiquidityProviderConfig {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
            asset_data: valence_library_utils::liquidity_utils::AssetData {
                asset1: dntrn_denom.clone(),
                asset2: usdc_denom.clone(),
            },
            max_spread,
        },
    };

    let lib_astroport_lper = builder.add_library(LibraryInfo::new(
        "astroport_lper".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportLper(astroport_lper_config.clone()),
    ));

    builder.add_link(&lib_astroport_lper, vec![&acc_provide_ready], EMPTY_VEC);

    // Configure the Return Forwarder
    // It will take USDC and dNTRN tokens from the Provide Ready account and return them to the Neutron DAO account.
    let return_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_provide_ready.clone(),
        output_addr: valence_library_utils::LibraryAccountType::Addr(neutron_dao_addr.clone()),
        forwarding_configs: vec![
            (
                cw_denom::UncheckedDenom::Native(usdc_denom.clone()),
                return_forwarder_max_amount
                    .parse()
                    .expect("return_forwarder_max_amount is not a valid number"),
            )
                .into(),
            (
                cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
                return_forwarder_max_amount
                    .parse()
                    .expect("return_forwarder_max_amount is not a valid number"),
            )
                .into(),
        ],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let lib_return_forwarder = builder.add_library(LibraryInfo::new(
        "return_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(return_forwarder_config.clone()),
    ));

    builder.add_link(&lib_return_forwarder, vec![&acc_provide_ready], EMPTY_VEC);

    // Authorizations
    // Create an authorization to batch and forward USDC/NTRN LP tokens
    let forward_usdc_ntrn_lp_func = AtomicFunctionBuilder::new()
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
        .with_function(forward_usdc_ntrn_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("forward_usdc_ntrn_lp_batch")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Create an authorization to withdraw NTRN/USDC LP tokens
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
        .with_label("withdraw_usdc_ntrn_liquidity")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Create an authorization to forward USDC tokens to the Astroport LPer
    let forward_usdc_to_provide_ready_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_usdc_to_provide_ready_forwarder.clone())
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
        .with_function(forward_usdc_to_provide_ready_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("forward_usdc_to_provide_ready_account")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Create an authorization to liquid stake NTRN tokens
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

    // Create an authorization to provide double sided liquidity
    let expected_pool_ratio_range =
        Some(valence_library_utils::liquidity_utils::DecimalRange::new(
            Decimal::from_str(expected_pool_ratio_min.as_str())
                .expect("expected_pool_ratio_min must be parsed into Decimal"),
            Decimal::from_str(expected_pool_ratio_max.as_str())
                .expect("expected_pool_ratio_max must be parsed into Decimal"),
        ));
    let provide_double_sided_liquidity_func = AtomicFunctionBuilder::new()
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
        .with_function(provide_double_sided_liquidity_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("provide_double_sided_liquidity")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Create an authorization to provide double sided liquidity in secure mode
    let secure_double_sided_lp_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_astroport_lper.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "provide_double_sided_liquidity".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(secure_double_sided_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_provide_double_sided_liquidity")
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

    // Create an authorization to provide single sided liquidity in secure mode
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
        .with_label("secure_provide_single_sided_liquidity")
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

    // Create an authorization to return unspent tokens from the provide ready account back to the Neutron DAO.
    // This authorization is in secure mode
    let secure_return_unspent_tokens_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_return_forwarder.clone())
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
        .with_function(secure_return_unspent_tokens_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_return_unspent_tokens")
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

    // Create an authorization to update the ntrn/usdc lp tokens forward config
    let secure_update_lp_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_usdc_lp_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "update_config".to_string(),
                params_restrictions: Some(vec![
                    ParamRestriction::MustBeIncluded(vec![
                        "update_config".to_string(),
                        "new_config".to_string(),
                    ]),
                    ParamRestriction::CannotBeIncluded(vec![
                        "update_config".to_string(),
                        "new_config".to_string(),
                        "input_addr".to_string(),
                    ]),
                    ParamRestriction::CannotBeIncluded(vec![
                        "update_config".to_string(),
                        "new_config".to_string(),
                        "output_addr".to_string(),
                    ]),
                ]),
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

    // Create an authorization to update the usdc forwarder config
    let secure_update_usdc_forwarder_config_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_usdc_to_provide_ready_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "update_config".to_string(),
                params_restrictions: Some(vec![
                    ParamRestriction::MustBeIncluded(vec![
                        "update_config".to_string(),
                        "new_config".to_string(),
                    ]),
                    ParamRestriction::CannotBeIncluded(vec![
                        "update_config".to_string(),
                        "new_config".to_string(),
                        "input_addr".to_string(),
                    ]),
                    ParamRestriction::CannotBeIncluded(vec![
                        "update_config".to_string(),
                        "new_config".to_string(),
                        "output_addr".to_string(),
                    ]),
                ]),
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
