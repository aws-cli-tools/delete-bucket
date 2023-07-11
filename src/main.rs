use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::Client;
use clap::Parser;
use console::style;
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

async fn delete_and_capture(client: &Client, bucket_name: &str) {
    if let Err(e) = delete_bucket::delete_bucket(client, bucket_name, &mut std::io::stdout()).await
    {
        eprintln!("Error deleting bucket: {}", e);
        std::process::exit(1);
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Opt::parse();

    let region_provider = delete_bucket::get_region_provider(args.region);

    let shared_config = delete_bucket::get_aws_config(args.profile, region_provider).await;

    let client = S3Client::new(&shared_config);
    if !args.force {
        if Confirm::new()
            .with_prompt(format!(
                "Are you certain you'd like to delete the {} S3 bucket?",
                style(&args.bucket).white().bold()
            ))
            .default(false)
            .interact()?
        {
            delete_and_capture(&client, &args.bucket).await;
        } else {
            println!("Cancelled");
        }
    } else {
        delete_and_capture(&client, &args.bucket).await;
    }

    Ok(())
}
