use anyhow::Result;
use clap::Parser;
use std::fmt::Debug;
use delete_bucket::{OutputType, StsClient};

#[derive(Debug, Parser)]
struct Opt {
    /// The AWS Region.
    #[arg(short, long)]
    region: Option<String>,

    /// Which profile to use.
    #[arg(short, long)]
    profile: Option<String>,

    /// Do not prompt for approval
    #[arg(short, long)]
    force: bool,

    /// Bucket to delete 
    #[arg(short, long)]
    bucket: Vec<String>,

    #[arg(value_enum)]
    #[arg(short, long,default_value_t=OutputType::String)]
    output_type: OutputType,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Opt::parse();

    let region_provider = delete_bucket::get_region_provider(args.region);

    let shared_config = delete_bucket::get_aws_config(args.profile, region_provider).await;

    let client = StsClient::new(&shared_config);
    delete_bucket::get_caller_identity(&client, args.output_type, &mut std::io::stdout()).await?;

    Ok(())
}
