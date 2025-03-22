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
    // Get program params from params.toml
    let owner = params.get("owner");
    // denoms
    let ntrn_denom = params.get("ntrn_denom");
    let dntrn_denom = params.get("dntrn_denom");
    // pool configuration
    let astroport_pool_addr = params.get("astroport_pool_addr");
    let pool_max_spread = params.get("pool_max_spread");
    let double_sided_min = params.get("double_sided_min");
    let double_sided_max = params.get("double_sided_max");
    // forwarder configuration
    let ntrn_forwarder_amount = params.get("ntrn_forwarder_amount");
    let dntrn_forwarder_amount = params.get("dntrn_forwarder_amount");
    let forwarder_interval_between_calls = params.get("forwarder_interval_between_calls");
    // allowed addresses
    let neutron_dao_addr = params.get("neutron_dao_addr");
    let security_dao_addr = params.get("security_dao_addr");
    let operator_list = params.get_array("operator_list");

    let permissioned_all_mode =
        valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
            valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                operator_list,
            ),
        );

    let mut builder = ProgramConfigBuilder::new("Bootstrap NTRN-dNTRN Production v1", &owner);

    // Domains
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm("neutron".to_string());

    /// Accounts
    /// We need one account to receive NTRN and dNTRN tokens
    let acc_receive = builder.add_account(AccountInfo::new(
        "receive_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
 
    /// Libraries
    /// This program requires two libraries:
    /// 1. A forwarder library to return excess NTRN and dNTRN tokens to the Neutron DAO
    /// 2. A liquidity provider library to provide liquidity to the pool and send LP tokens to the Neutron DAO
    
    // Forwarder library to return excess NTRN and dNTRN tokens to the Neutron DAO
    let return_forwarder_constraints =
        if forwarder_interval_between_calls.is_empty() || forwarder_interval_between_calls == "0" {
            None
        } else {
            Some(cw_utils::Duration::Time(
                forwarder_interval_between_calls
                    .parse()
                    .expect("forwarder_interval_between_calls is not a valid number"),
            ))
        };
    let return_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_receive.clone(),
        output_addr: LibraryAccountType::Addr(neutron_dao_addr.clone()),
        forwarding_configs: vec![
            (
                cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
                ntrn_forwarder_amount
                    .parse()
                    .expect("ntrn_forwarder_amount is not a valid number"),
            )
                .into(),
            (
                cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
                dntrn_forwarder_amount
                    .parse()
                    .expect("dntrn_forwarder_amount is not a valid number"),
            )
                .into(),
        ],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(
            return_forwarder_constraints,
        ),
    };

    let lib_return_forwarder = builder.add_library(LibraryInfo::new(
        "return_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(return_forwarder_config.clone()),
    ));

    builder.add_link(&lib_return_forwarder, vec![&acc_receive], EMPTY_VEC);

    // We need a liquidity provider library to provide liquidity to the pool
    let max_spread = if pool_max_spread.is_empty() {
        None
    } else {
        Some(
            Decimal::from_str(pool_max_spread.as_str())
                .expect("pool_max_spread must be valid Decimal string"),
        )
    };
    let astroport_lper_config = valence_astroport_lper::msg::LibraryConfig {
        input_addr: acc_receive.clone(),
        output_addr: LibraryAccountType::Addr(neutron_dao_addr.clone()),
        pool_addr: astroport_pool_addr.clone(),
        lp_config: valence_astroport_lper::msg::LiquidityProviderConfig {
            pool_type: valence_astroport_utils::PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
            asset_data: valence_library_utils::liquidity_utils::AssetData {
                asset1: dntrn_denom.clone(),
                asset2: ntrn_denom.clone(),
            },
            max_spread,
        },
    };

    let lib_astroport_lper = builder.add_library(LibraryInfo::new(
        "astroport_lper".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceAstroportLper(astroport_lper_config.clone()),
    ));

    builder.add_link(&lib_astroport_lper, vec![&acc_receive], EMPTY_VEC);

    /// We need to build the subroutines and create authorizations for them
    /// 1. Send tokens to DAO
    /// 2. Provide double sided liquidity
    /// 3. Provide double sided liquidity by DAO
    /// 4. Provide single sided liquidity by DAO
    /// 5. Update return forwarder config

    // Send tokens to DAO
    let send_tokens_to_dao_subroutine = AtomicSubroutineBuilder::new()
        .with_function(AtomicFunctionBuilder::new(  )
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
            .build()
        )
        .build();

    // Tokens can only be sent to the DAO by the security DAO
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_send_tokens_to_dao")
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![security_dao_addr.clone()],
                ),
            ),
        )
        .with_subroutine(send_tokens_to_dao_subroutine)
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

    let double_sided_lp_subroutine = AtomicSubroutineBuilder::new()
        .with_function(double_sided_lp_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("double_sided_lp")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Provide double sided liquidity by security DAO
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
        .with_label("secure_double_sided_lp")
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
        .with_label("secure_single_sided_lp")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update return forwarder config
    let update_return_forwarder_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_return_forwarder.clone())
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
        .with_function(update_return_forwarder_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone(), security_dao_addr.clone()],
                ),
            ),
        )
        .with_label("secure_update_return_forwarder_config")
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    builder.build()
}
