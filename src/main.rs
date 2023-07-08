use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use clap::Parser;
use dialoguer::Confirm;
use std::fmt::Debug;

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
    bucket: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Opt::parse();

    let region_provider = delete_bucket::get_region_provider(args.region);

    let shared_config = delete_bucket::get_aws_config(args.profile, region_provider).await;

    let client = S3Client::new(&shared_config);
    if args.force != true {
        if Confirm::new()
            .with_prompt(format!(
                "Do you want to continue with the deletion of {}",
                args.bucket
            ))
            .default(false)
            .interact()?
        {
            if let Err(e) = delete_bucket::delete_bucket(&client, &args.bucket, &mut std::io::stdout()).await {
                eprintln!("Error deleting bucket: {}", e);
                std::process::exit(1);
            }
        } else {
            println!("Cancelled");
        }
    } else {
        if let Err(e) = delete_bucket::delete_bucket(&client, &args.bucket, &mut std::io::stdout()).await {
            eprintln!("Error deleting bucket: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
