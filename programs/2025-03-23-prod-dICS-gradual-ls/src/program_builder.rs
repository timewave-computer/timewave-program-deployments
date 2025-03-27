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

pub fn program_builder(params: deployer_lib::ProgramParams) -> ProgramConfig {
    // program params

    let owner = params.get("owner");
    let ntrn_denom = params.get("ntrn_denom");
    let drop_liquid_staker_addr = params.get("drop_liquid_staker_addr");
    let max_amount_to_forward = params.get("max_amount_to_forward");
    let interval_seconds_between_batches = params.get("interval_seconds_between_batches");
    let neutron_dao_addr = params.get("neutron_dao_addr");
    let security_dao_addr = params.get("security_dao_addr");
    let operator_list = params.get_array("operator_list");

    let permissioned_all_mode =
        valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
            valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                operator_list,
            ),
        );

    // Domains
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm("neutron".to_string());

    let mut builder = ProgramConfigBuilder::new("Valence dICS Program 3: Gradual LS", &owner);

    // Receiver account
    let acc_drip = builder.add_account(AccountInfo::new(
        "receiver_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    // Interim account
    let acc_ls = builder.add_account(AccountInfo::new(
        "interim_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Add the drip forwarder library
    let drip_forwarder_constraints =
        if interval_seconds_between_batches.is_empty() || interval_seconds_between_batches == "0" {
            None
        } else {
            Some(cw_utils::Duration::Time(
                interval_seconds_between_batches
                    .parse()
                    .expect("interval_seconds_between_batches is not a valid number"),
            ))
        };
    let drip_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_drip.clone(),
        output_addr: acc_ls.clone(),
        forwarding_configs: vec![(
            cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
            max_amount_to_forward
                .parse()
                .expect("max_amount_to_forward is not a valid number"),
        )
            .into()],
        forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(
            drip_forwarder_constraints,
        ),
    };

    let lib_drip_forwarder = builder.add_library(LibraryInfo::new(
        "drip_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(drip_forwarder_config.clone()),
    ));

    builder.add_link(&lib_drip_forwarder, vec![&acc_drip], vec![&acc_ls]);

    // Add the drop liquid staker library
    let drop_liquid_staker_config = valence_drop_liquid_staker::msg::LibraryConfig {
        input_addr: acc_ls.clone(),
        output_addr: neutron_dao_addr.as_str().into(),
        liquid_staker_addr: drop_liquid_staker_addr,
        denom: ntrn_denom,
    };

    let lib_drop_liquid_staker = builder.add_library(LibraryInfo::new(
        "drop_liquid_staker".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceDropLiquidStaker(drop_liquid_staker_config.clone()),
    ));

    builder.add_link(&lib_drop_liquid_staker, vec![&acc_ls], EMPTY_VEC);

    // Authorizations
    // Drip forwarder authorization
    let drip_forward_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_drip_forwarder.clone())
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
        .with_label("forward_batch")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Liquid stake authorization
    let liquid_stake_function = AtomicFunctionBuilder::new()
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
        .with_function(liquid_stake_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("liquid_stake_batch")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update forward config
    let secure_update_forward_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_drip_forwarder.clone())
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
        .with_function(secure_update_forward_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_update_forwarder_config")
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
