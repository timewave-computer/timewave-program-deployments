mod program_builder;

use std::error::Error;

use program_builder::program_builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    deployer_lib::main(file!(), program_builder).await
}
