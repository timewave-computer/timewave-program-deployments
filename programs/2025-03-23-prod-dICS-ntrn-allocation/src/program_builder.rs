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
    // Get program params
    let owner = params.get("owner");
    let ntrn_denom = params.get("ntrn_denom");

    // Valence Program 2: Instant liquid stake
    let vp2_instant_ls_receiver_address = params.get("vp1_instant_ls_receiver_address");
    let vp2_instant_ls_amount = params.get("vp1_instant_ls_amount");

    // Valence Program 3: Gradual liquid stake
    let vp3_gradual_ls_receiver_address = params.get("vp2_gradual_ls_receiver_address");
    let vp3_gradual_ls_amount = params.get("vp2_gradual_ls_amount");

    // Valence Program 4: Bootstrap NTRN-dNTRN liquidity
    let vp4_bootstrap_liquidity_receiver_address =
        params.get("vp4_bootstrap_liquidity_receiver_address");
    let vp4_bootstrap_liquidity_receiver_amount =
        params.get("vp4_bootstrap_liquidity_receiver_amount");

    // Neutron DAO address
    let neutron_dao_addr = params.get("neutron_dao_addr");
    // Security DAO address
    let security_dao_addr = params.get("security_dao_addr");
    // List of address that are allowed to execute low security operations
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

    // Write your program
    let mut builder = ProgramConfigBuilder::new("Valence dICS Program 1: NTRN Allocation", &owner);

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
                vp2_instant_ls_receiver_address.as_str(),
                valence_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                    vp2_instant_ls_amount
                        .parse()
                        .expect("Failed to parse vp2_instant_ls_amount as Uint128"),
                ),
            ),
            valence_splitter_library::msg::UncheckedSplitConfig::new(
                cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
                vp3_gradual_ls_receiver_address.as_str(),
                valence_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                    vp3_gradual_ls_amount
                        .parse()
                        .expect("Failed to parse vp3_gradual_ls_amount as Uint128"),
                ),
            ),
            valence_splitter_library::msg::UncheckedSplitConfig::new(
                cw_denom::UncheckedDenom::Native(ntrn_denom.clone()),
                vp4_bootstrap_liquidity_receiver_address.as_str(),
                valence_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                    vp4_bootstrap_liquidity_receiver_amount.parse().expect(
                        "Failed to parse vp4_bootstrap_liquidity_receiver_amount as Uint128",
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
