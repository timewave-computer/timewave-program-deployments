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
    let ntrn_denom = params.get("ntrn_denom");
    let dntrn_denom = params.get("dntrn_denom");
    let astroport_lp_denom = params.get("astroport_lp_denom");
    let astroport_pool_addr = params.get("astroport_pool_addr");
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

    let mut builder = ProgramConfigBuilder::new(owner);

    // Domains
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm("neutron".to_string());

    // Accounts
    let acc_ntrn_receive = builder.add_account(AccountInfo::new(
        "receive_ntrn_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_dntrn_receive = builder.add_account(AccountInfo::new(
        "receive_dntrn_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_lp = builder.add_account(AccountInfo::new(
        "lp_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_lp_token_holder = builder.add_account(AccountInfo::new(
        "lp_token_holder_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_unlock = builder.add_account(AccountInfo::new(
        "unlock_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Libraries
    // Ntrn forwarder library
    let ntrn_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_ntrn_receive.clone(),
        output_addr: acc_lp.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(Some(
            cw_utils::Duration::Time(60 * 60),
        )),
    };

    let lib_ntrn_forwarder = builder.add_library(LibraryInfo::new(
        "ntrn_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(ntrn_forwarder_config.clone()),
    ));

    builder.add_link(&lib_ntrn_forwarder, vec![&acc_ntrn_receive], vec![&acc_lp]);

    // Ntrn forwarder library
    let dntrn_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_dntrn_receive.clone(),
        output_addr: acc_lp.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(Some(
            cw_utils::Duration::Time(60 * 60),
        )),
    };

    let lib_dntrn_forwarder = builder.add_library(LibraryInfo::new(
        "dntrn_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(dntrn_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_dntrn_forwarder,
        vec![&acc_dntrn_receive],
        vec![&acc_lp],
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
        input_addr: acc_lp.clone(),
        output_addr: acc_lp_token_holder.clone(),
        pool_addr: astroport_pool_addr.clone(),
        lp_config: valence_astroport_lper::msg::LiquidityProviderConfig {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
            asset_data: valence_library_utils::liquidity_utils::AssetData {
                asset1: ntrn_denom.clone(),
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
        vec![&acc_lp],
        vec![&acc_lp_token_holder],
    );

    // lp to dao forwarder
    let lp_token_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_lp_token_holder.clone(),
        output_addr: LibraryAccountType::Addr(neutron_dao_addr.clone()),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(astroport_lp_denom.clone()),
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
        vec![&acc_lp_token_holder],
        EMPTY_VEC,
    );

    // unlock forwarder
    let unlock_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_lp_token_holder.clone(),
        output_addr: acc_unlock.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(Some(
            cw_utils::Duration::Time(60 * 60),
        )),
    };

    let lib_unlock_forwarder = builder.add_library(LibraryInfo::new(
        "to_unlock_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(unlock_forwarder_config.clone()),
    ));

    builder.add_link(
        &lib_unlock_forwarder,
        vec![&acc_lp_token_holder],
        vec![&acc_unlock],
    );

    // astroport withdrawer
    let astropo_withdrawer_config = valence_astroport_withdrawer::msg::LibraryConfig {
        input_addr: acc_unlock.clone(),
        output_addr: LibraryAccountType::Addr(neutron_dao_addr.clone()),
        pool_addr: astroport_pool_addr,
        withdrawer_config: valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
            asset_data: valence_library_utils::liquidity_utils::AssetData {
                asset1: ntrn_denom.clone(),
                asset2: dntrn_denom.clone(),
            },
        },
    };

    let lib_astrport_withdrawer = builder.add_library(LibraryInfo::new(
        "astroport_withdrawer".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportWithdrawer(astropo_withdrawer_config.clone()),
    ));

    builder.add_link(&lib_astrport_withdrawer, vec![&acc_unlock], EMPTY_VEC);

    // Authorizations

    // Forward ntrn
    let forward_ntrn_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_forwarder.clone())
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
        .with_function(forward_ntrn_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("forward_ntrn")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Forward dntrn
    let forward_dntrn_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_dntrn_forwarder.clone())
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
        .with_function(forward_dntrn_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("forward_dntrn")
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

    // Forward lp tokens to neutron dao
    let forward_lp_to_dao_func = AtomicFunctionBuilder::new()
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
        .with_contract_address(lib_lp_token_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "update_config".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "update_config".to_string(),
                    "forwarding_configs".to_string(),
                ])]),
            },
        })
        .build();
    let forward_lp_to_unlock_func = AtomicFunctionBuilder::new()
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
    let withdraw_from_astro_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_astrport_withdrawer.clone())
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

    // Update ntrn forward config
    let update_ntrn_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_ntrn_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "update_config".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(update_ntrn_forward_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_label("update_ntrn_forward_config")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update dntrn forward config
    let update_dntrn_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_dntrn_forwarder.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "update_config".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(update_dntrn_forward_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_label("update_dntrn_forward_config")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    builder.build()
}
