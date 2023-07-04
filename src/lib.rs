use anyhow::{Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_sdk_sts::{config::Region};
use aws_types::SdkConfig;
use clap::ValueEnum;
use log::info;
use serde_json::json;
use std::fmt::Debug;
use tokio_stream::StreamExt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputType {
    /// Output as json
    Json,
    /// Output as regular string
    String,
}

pub fn get_region_provider(region: Option<String>) -> RegionProviderChain {
    info!("Getting region details");

    RegionProviderChain::first_try(region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-west-2"))
}

pub async fn get_aws_config(
    profile: Option<String>,
    region_provider: RegionProviderChain,
) -> SdkConfig {
    if let Some(p) = profile {
        info!("Using profile - {}", p);
        aws_config::from_env()
            .region(region_provider)
            .profile_name(p)
            .load()
            .await
    } else {
        info!("Using default profile");
        aws_config::from_env().region(region_provider).load().await
    }
}
pub async fn delete_objects(
    client: &Client,
    bucket_name: &str,
    output_type: OutputType,
    mut writer: impl std::io::Write,
) -> Result<()> {
    info!("Calling 'list_objects_v2 to pull objects to delete'");
    let paginator = client.list_objects_v2().bucket(bucket_name).into_paginator().send();
    let objects: Vec<aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output> = paginator.collect::<Result<Vec<_>, _>>().await?;
    objects.
    info!("Successful call");
    let account_id = response.account().unwrap_or_default();
    let user_arn = response.arn().unwrap_or_default();

    info!("Output type is {:?}", output_type);
    match output_type {
        OutputType::String => {
            writeln!(writer, "AccountId = {}", account_id)?;
            writeln!(writer, "UserARN = {}", user_arn)?;
        }
        OutputType::Json => {
            let result = json!({
                "accountId": account_id,
                "UserARN": user_arn,
            });
            writeln!(writer, "{}", result)?;
        }
    }

    Ok(())
}
