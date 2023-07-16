use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use clap::Parser;
use console::style;
use dialoguer::Confirm;
use std::fmt::Debug;

#[derive(Debug, Parser)]
/// Delete S3 🪣. Use with caution!
#[command(version, about, long_about = None)]
#[clap(
    after_help = "➡️  Looking for more OSS tools for AWS, visit us at https://github.com/aws-cli-tools"
)]
struct DeleteBucketOpt {
    #[clap(flatten)]
    base: aws_cli_lib::Opt,

    /// Do not prompt for approval
    #[arg(short, long)]
    force: bool,

    #[arg(short, long, num_args(1..))]
    #[arg(help=format!("Buckets to delete, seperate with space, for example {} {} ", style(env!("CARGO_PKG_NAME")).white().bold(), style("-b bucket1 bucket2").white().bold()))]
    buckets: Vec<String>,
}

async fn delete_and_capture(client: &S3Client, bucket_name: &str) -> bool {
    if let Err(e) = delete_bucket::delete_bucket(client, bucket_name, &mut std::io::stdout()).await
    {
        eprintln!("Error deleting bucket: {}", e);
        return true;
    }

    false
}
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = DeleteBucketOpt::parse();

    let region_provider = aws_cli_lib::get_region_provider(args.base.region);

    let shared_config = aws_cli_lib::get_aws_config(args.base.profile, region_provider).await;

    let client = S3Client::new(&shared_config);
    let mut should_exit_with_error = false;
    for bucket in args.buckets {
        if !args.force {
            if Confirm::new()
                .with_prompt(format!(
                    "Are you certain you'd like to delete the {} S3 bucket?",
                    style(&bucket).white().bold()
                ))
                .default(false)
                .interact()?
            {
                should_exit_with_error =
                    delete_and_capture(&client, &bucket).await || should_exit_with_error;
            } else {
                println!("Skipping {}", style(&bucket).white().bold());
            }
        } else {
            should_exit_with_error =
                delete_and_capture(&client, &bucket).await || should_exit_with_error;
        }
    }

    if should_exit_with_error {
        std::process::exit(1);
    }
    Ok(())
}
