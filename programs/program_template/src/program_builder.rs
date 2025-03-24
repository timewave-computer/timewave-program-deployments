use std::str::FromStr;

use cosmwasm_std::Uint128;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_forwarder_library::msg::ForwardingConstraints;
use valence_library_utils::denoms::UncheckedDenom;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};

// Example program that move funds from first account to second account and vice versa
//
// We first take our parameters to get the owner, denom and max_forward_amount.
// Those are set as parameters because they might change from 1 deployment environment to another (mainnet, testnet, etc)
//
// We then create 2 accounts, one for the first account and one for the second account.
//
// We then create 2 libraries, first forwarder for first account to forward funds to the second account and
// second forwarder for the second account to forward funds to the first account.
//
// We then create 2 authorizations, one for the first account to forward funds to the second account and
// one for the second account to forward funds to the first account.
//
// The program config is then built

/// Write your program using the program builder
pub fn program_builder(params: deployer_lib::ProgramParams) -> ProgramConfig {
    //---- program params ----//
    // Owner of the program
    let owner = params.get("owner");
    // Denom to use for forwarding
    let denom = params.get("denom");
    // Max amount to forward
    let max_forward_amount = params.get("max_forward_amount");

    //---- Set builder ----//
    let mut builder = ProgramConfigBuilder::new("example-program", owner.as_str());

    //---- Domains ----//
    // Neutron domain
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm("neutron".to_string());

    //---- Accounts ----//
    // First account
    let acc_first = builder.add_account(AccountInfo::new(
        "first_account".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    // Second account
    let acc_second = builder.add_account(AccountInfo::new(
        "second_account".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    //---- Libraries ----//
    // Forward funds from first account to second account
    let first_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_first.clone(),
        output_addr: acc_second.clone(),
        forwarding_configs: vec![valence_forwarder_library::msg::UncheckedForwardingConfig {
            denom: UncheckedDenom::Native(denom.clone()),
            max_amount: Uint128::from_str(max_forward_amount.as_str()).unwrap(),
        }],
        forwarding_constraints: ForwardingConstraints::new(None),
    };

    let lib_first_forwarder = builder.add_library(LibraryInfo::new(
        "first_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(first_forwarder_config),
    ));

    builder.add_link(&lib_first_forwarder, vec![&acc_first], vec![&acc_second]);

    // Forward funds from second account to first account
    let second_forwarder_config = valence_forwarder_library::msg::LibraryConfig {
        input_addr: acc_second.clone(),
        output_addr: acc_first.clone(),
        forwarding_configs: vec![valence_forwarder_library::msg::UncheckedForwardingConfig {
            denom: UncheckedDenom::Native(denom),
            max_amount: Uint128::from_str(max_forward_amount.as_str()).unwrap(),
        }],
        forwarding_constraints: ForwardingConstraints::new(None),
    };

    let lib_second_forwarder = builder.add_library(LibraryInfo::new(
        "second_forwarder".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceForwarderLibrary(second_forwarder_config),
    ));

    builder.add_link(&lib_second_forwarder, vec![&acc_second], vec![&acc_first]);

    //---- Authorizations ----//
    // First authorization to forward funds from the first account to the second account
    let action_label = "first_forward";
    let function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_first_forwarder.clone())
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
        .with_function(function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label(action_label)
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Second authorization to forward funds from the second account to the first account
    let action_label = "second_forward";
    let function = AtomicFunctionBuilder::new()
        .with_contract_address(lib_second_forwarder.clone())
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
        .with_function(function)
        .build();
    let authorization = AuthorizationBuilder::new()
        .with_label(action_label)
        .with_subroutine(subroutine)
        .build();

    builder.add_authorization(authorization);

    // Build program config
    builder.build()
}
