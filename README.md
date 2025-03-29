# Timewave Program Deployments

This repository contains scripts and program definitions for [Valence Programs](https://docs.valence.zone) that the Timewave team has deployed.

## Overview

This repository includes:
- Templates for creating new programs
- Environment-specific configuration management
- Build and deployment scripts
- Documentation for existing deployments

## Index of Deployments
- [Neutron dICS Deployments](./Neutron_dICS_Programs.md)

## Getting Started
Please follow this guide to create and deploy your own program. 

### Prerequisites
- Rust toolchain
- Familiarity with [Valence](https://docs.valence.zone)
- A wallet funded with tokens for gas

### Set up `.env`

You must include `.env` file in the root directory with the mnemonic of a funded wallet that will be used to deploy programs.

```env
# The mnemonic that will be used by the program manager to deploy the program on chains
MANAGER_MNEMONIC=""
```

### Clone the template

A [template](./programs/program_template/) is provided to help you get started. You can copy the directory in its entirety and modify it to build and deploy your program.

Don't forget to rename the new program directory and the program name in the `Cargo.toml` file.

### Program builder

In your new program directory you will find `src/program_builder.rs` file, this is the file that you will modify to create your program. You will be using the Rust builder pattern to write the program.

### Environment specific program parameters

Your program may need parameters that are unique for a specific deployment environment. For example, you may want to use particular parameters for a testnet deployment and different parameters for mainnet, while building the program from the *same* `src/program_builder.rs` file.

You can provide these parameters in the `program_params/` directory.

You should include a file for each environment. The file name should be the environment name. For example, you might have `program_params/local.toml` or `program_params/mainnet.toml`.

For every environment you must make sure that there is an equivalent directory in `manager_configs`.

Any parameter that is included there will be available in the program builder function, and can be retrieved using the `.get(String)` function, Example: `params.get("my_param")`.

### Build and deploy

You can build and deploy your program using the following command:

```bash
cargo run -p <PROGRAM_NAME> -- --target-env <ENVIRONMENT>
```

Use the program name you gave your program in `Cargo.toml` file.

### Output

After running the script, you will find the output in the `output/` directory.

Each deployed program will have its own directory with the date and time of the deployment. Inside each directory you will find:

- `instantiated-program-config.json`: Which includes all the addresses of the contracts of the deployed program.
- `raw-program-config.json`: Which includes the raw program config that resulted from the builder before instantiation.

By default each deployed program directory is ignored in git, you can remove the ignore rule in the `.gitignore` file if you want to keep the output in your cloned repository.

## Contributing

Please ensure your changes follow the existing patterns and include appropriate documentation updates.