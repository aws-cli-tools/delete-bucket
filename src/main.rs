use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use clap::Parser;
use console::style;
use dialoguer::Confirm;
use std::fmt::Debug;

#[derive(Debug, Parser)]
struct DeleteBucketOpt {
    #[clap(flatten)]
    base: aws_cli_lib::Opt,

    /// Do not prompt for approval
    #[arg(short, long)]
    force: bool,

    /// Bucket to delete
    #[arg(short, long)]
    bucket: String,
}

async fn delete_and_capture(client: &S3Client, bucket_name: &str) {
    if let Err(e) = delete_bucket::delete_bucket(client, bucket_name, &mut std::io::stdout()).await
    {
        eprintln!("Error deleting bucket: {}", e);
        std::process::exit(1);
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = DeleteBucketOpt::parse();

    let region_provider = aws_cli_lib::get_region_provider(args.base.region);

    let shared_config = aws_cli_lib::get_aws_config(args.base.profile, region_provider).await;

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

