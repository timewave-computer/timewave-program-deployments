use std::str::FromStr;

use cosmwasm_std::Decimal;
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
    let ntrn_denom = params.get("ntrn_denom");
    let dntrn_denom = params.get("dntrn_denom");
    let bootstrap_ntrn_dntrn_receive_addr = params.get("bootstrap_ntrn_dntrn_receive_addr");
    let drop_liquid_staker_addr = params.get("drop_liquid_staker_addr");
    let drop_liquid_unstaker_addr = params.get("drop_liquid_unstaker_addr");
    let drop_withdrawal_manager_addr = params.get("drop_withdrawal_manager_addr");
    let voucher_addr = params.get("voucher_addr");
    let neutron_dao_addr = params.get("neutron_dao_addr");
    let security_dao_addr = params.get("security_dao_addr");
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

    let mut builder = ProgramConfigBuilder::new("neutron stake drop test", &owner);

    // Input for tokens to drip
    let acc_drip = builder.add_account(AccountInfo::new(
        "drip_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    // Input for tokens to liquid stake
    let acc_ls = builder.add_account(AccountInfo::new(
        "ls_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_interim = builder.add_account(AccountInfo::new(
        "interim_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_stake_holder = builder.add_account(AccountInfo::new(
        "stake_holder_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_unstake = builder.add_account(AccountInfo::new(
        "unstake_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Add the drip forwarder library
    let drip_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_drip.clone(),
        output_addr: acc_ls.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(Some(
            cw_utils::Duration::Time(60 * 60),
        )),
    };

    let drip_forwarder_library = builder.add_library(LibraryInfo::new(
        "drip_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(drip_forwarder_config.clone()),
    ));

    builder.add_link(&drip_forwarder_library, vec![&acc_drip], vec![&acc_ls]);

    // Add the drop liquid staker library
    let drop_liquid_staker_config = valence_drop_liquid_staker::msg::LibraryConfig {
        input_addr: acc_ls.clone(),
        output_addr: acc_interim.clone(),
        liquid_staker_addr: drop_liquid_staker_addr,
        denom: ntrn_denom,
    };

    let drop_liquid_staker_library = builder.add_library(LibraryInfo::new(
        "drop_liquid_staker".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceDropLiquidStaker(drop_liquid_staker_config.clone()),
    ));

    builder.add_link(
        &drop_liquid_staker_library,
        vec![&acc_ls],
        vec![&acc_interim],
    );

    // Liquid staked token splitter
    let split_ls_token_config = valence_splitter_library::msg::LibraryConfig {
        input_addr: acc_interim.clone(),
        splits: vec![
            valence_splitter_library::msg::UncheckedSplitConfig::new(
                cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
                acc_stake_holder.clone(),
                valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                    Decimal::from_str("0.5").expect("Decimal::from_str failed"),
                ),
            ),
            valence_splitter_library::msg::UncheckedSplitConfig::new(
                cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
                bootstrap_ntrn_dntrn_receive_addr.as_str(),
                valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                    Decimal::from_str("0.5").expect("Decimal::from_str failed"),
                ),
            ),
        ],
    };

    let split_ls_token_library = builder.add_library(LibraryInfo::new(
        "liquid_staked_splitter".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(split_ls_token_config.clone()),
    ));

    builder.add_link(
        &split_ls_token_library,
        vec![&acc_interim],
        vec![&acc_stake_holder],
    );

    // Liquid staked token forwarder
    let ls_token_forwader_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_stake_holder.clone(),
        output_addr: acc_unstake.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
            1_000_000_u128,
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(None),
    };

    let ls_token_forwader_library = builder.add_library(LibraryInfo::new(
        "liquid_staked_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(ls_token_forwader_config.clone()),
    ));

    builder.add_link(
        &ls_token_forwader_library,
        vec![&acc_stake_holder],
        vec![&acc_unstake],
    );

    // Unstake liquid staked token
    let drop_unstake_config = valence_drop_liquid_unstaker::msg::LibraryConfig {
        input_addr: acc_unstake.clone(),
        output_addr: valence_library_utils::LibraryAccountType::Addr(neutron_dao_addr.clone()),
        liquid_unstaker_addr: drop_liquid_unstaker_addr,
        withdrawal_manager_addr: drop_withdrawal_manager_addr,
        voucher_addr,
        denom: dntrn_denom,
    };

    let drop_unstake_library = builder.add_library(LibraryInfo::new(
        "drop_unstaker".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceDropLiquidUnstaker(drop_unstake_config.clone()),
    ));

    builder.add_link(&drop_unstake_library, vec![&acc_unstake], EMPTY_VEC);

    // Authorizations

    // Drip forwader authorization
    let drip_forward_function = AtomicFunctionBuilder::new()
        .with_contract_address(drip_forwarder_library.clone())
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
        .with_function(drip_forward_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("drip_forward")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Liquid stake and split authorization
    // Liquid stake function
    let liquid_stake_function = AtomicFunctionBuilder::new()
        .with_contract_address(drop_liquid_staker_library.clone())
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

    // split function
    let split_function = AtomicFunctionBuilder::new()
        .with_contract_address(split_ls_token_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "split".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(liquid_stake_function)
        .with_function(split_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("liquid_stake_and_split")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Unlock dNTRN
    let staked_token_forward_function = AtomicFunctionBuilder::new()
        .with_contract_address(ls_token_forwader_library.clone())
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
        .with_function(staked_token_forward_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("staked_token_forward")
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

    // Unstake dNTRN
    let unstake_function = AtomicFunctionBuilder::new()
        .with_contract_address(drop_unstake_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "unstake".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(unstake_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("unstake")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // withdraw dNTRN
    let withdraw_function = AtomicFunctionBuilder::new()
        .with_contract_address(drop_unstake_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "withdraw".to_string(),
                    "token_id".to_string(),
                ])]),
            },
        })
        .build();

    let subroutine = AtomicSubroutineBuilder::new()
        .with_function(withdraw_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("withdraw")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update split config
    let update_split_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(split_ls_token_library.clone())
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
        .with_function(update_split_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("update_split_config")
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

    // Update forward config
    let update_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(drip_forwarder_library.clone())
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
        .with_function(update_forward_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("update_forward_config")
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

    // Update dntrn forward config
    let update_dntrn_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(ls_token_forwader_library.clone())
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
        .with_function(update_dntrn_forward_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("update_dntrn_forward_config")
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
