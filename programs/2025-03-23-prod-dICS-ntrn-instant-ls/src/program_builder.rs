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
    let neutron_dao_addr = params.get("neutron_dao_addr");

    let vp4_receiver_split_normalized_fraction =
        params.get("vp4_receiver_split_normalized_fraction");

    // Addresses
    let vp4_bootstrap_liquidity_receiver_addr = params.get("vp4_bootstrap_liquidity_receiver_addr");
    let drop_liquid_staker_addr = params.get("drop_liquid_staker_addr");
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

    let mut builder =
        ProgramConfigBuilder::new("Valence dICS Program 2: Instant Liquid Stake NTRN", &owner);

    // Receiver account of the program
    let acc_receiver = builder.add_account(AccountInfo::new(
        "ntrn_receiver".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let acc_interim = builder.add_account(AccountInfo::new(
        "interim_acc".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    // Add the drop liquid staker library
    let drop_liquid_staker_config = valence_drop_liquid_staker::msg::LibraryConfig {
        input_addr: acc_receiver.clone(),
        output_addr: acc_interim.clone(),
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
        vec![&acc_receiver],
        vec![&acc_interim],
    );

    // Liquid staked token splitter
    let split_ls_token_config = valence_splitter_library::msg::LibraryConfig {
        input_addr: acc_interim.clone(),
        splits: vec![valence_splitter_library::msg::UncheckedSplitConfig::new(
            cw_denom::UncheckedDenom::Native(dntrn_denom.clone()),
            vp4_bootstrap_liquidity_receiver_addr.as_str(),
            valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                vp4_receiver_split_normalized_fraction
                    .parse()
                    .expect("Failed to parse vp4_receiver_split_normalized_fraction as Decimal"),
            ),
        )],
    };

    let lib_split_ls_token = builder.add_library(LibraryInfo::new(
        "liquid_staked_splitter".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(split_ls_token_config.clone()),
    ));

    builder.add_link(&lib_split_ls_token, vec![&acc_interim], EMPTY_VEC);

    // Authorizations

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
        .with_label("liquid_stake")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // split function
    let split_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_split_ls_token.clone())
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
        .with_function(split_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("split_to_provide")
        .with_mode(permissioned_all_mode.clone())
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Update split config
    let secure_update_split_config_function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_split_ls_token.clone())
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
        .with_function(secure_update_split_config_function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label("secure_update_split_config")
        .with_mode(
            valence_authorization_utils::authorization::AuthorizationModeInfo::Permissioned(
                valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(
                    vec![neutron_dao_addr.clone()],
                ),
            ),
        )
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    builder.build()
}
