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
    let bootstrap_program_receiver = params.get("bootstrap_program_receiver");
    let stake_ntrn_program_receiver = params.get("stake_ntrn_program_receiver");
    let ntrn_split_to_bootstrap_program_fixed_amount = params.get("ntrn_split_to_bootstrap_program_fixed_amount");
    let ntrn_split_to_stake_program_fixed_amount = params.get("ntrn_split_to_stake_program_fixed_amount");
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

    // Write your program
    let mut builder = ProgramConfigBuilder::new("program template", &owner);

    // Receiver account of the program
    let acc_receiver = builder.add_account(AccountInfo::new(
        "ntrn_receiver".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Splitter config
    let split_ntrn_config = valence_splitter_library::msg::LibraryConfig {
        input_addr: acc_receiver.clone(),
        splits: vec![
            valence_splitter_library::msg::UncheckedSplitConfig::new(
                cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
                bootstrap_program_receiver.as_str(),
                valence_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                    ntrn_split_to_bootstrap_program_fixed_amount.parse().expect(
                        "Failed to parse ntrn_split_to_bootstrap_program_fixed_amount as Decimal",
                    ),
                ),
            ),
            valence_splitter_library::msg::UncheckedSplitConfig::new(
                cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
                stake_ntrn_program_receiver.as_str(),
                valence_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                    ntrn_split_to_stake_program_fixed_amount
                        .parse()
                        .expect(
                        "Failed to parse ntrn_split_to_stake_program_fixed_amount as Decimal",
                    ),
                ),
            ),
        ],
    };

    let lib_split_ntrn = builder.add_library(LibraryInfo::new(
        "split_ntrn_library".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(split_ntrn_config.clone()),
    ));

    builder.add_link(&lib_split_ntrn, vec![&acc_receiver], EMPTY_VEC);


    // Authorizations
    // Split ntrn
    let split_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_split_ntrn.clone())
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
        .with_function(split_func)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("split_ntrn")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update split config
    let update_split_config_func = AtomicFunctionBuilder::new()
        .with_contract_address(lib_split_ntrn.clone())
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
        .with_function(update_split_config_func)
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

    builder.build()
}
