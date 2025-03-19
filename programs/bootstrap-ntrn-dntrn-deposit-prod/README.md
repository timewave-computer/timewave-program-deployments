# Program builder

This is a single program builder, structure:

- `output/` - Output directory for deployed program
- `src/` - Program source code
- `src/main.rs` - entry point to the script
- `src/program.rs` - Program builder code, this is rust helper that allows you to build the program config, this is the only file that should be modified to deploy a program.
- `program_params/` - Program parameters that will injected into the program builder function, each environment will have its own program parameters toml file, name with the environment name, Example: `program_params/local.toml` or `program_params/mainnet.toml`
