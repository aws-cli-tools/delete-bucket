use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::operation::list_objects_v2::{ListObjectsV2Error, ListObjectsV2Output};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use aws_sdk_s3::Client;
use aws_sdk_sts::client::customize::Response;
use aws_sdk_sts::config::Region;
use aws_sdk_sts::error::SdkError;
use aws_types::SdkConfig;
use futures::stream::FuturesUnordered;
use indicatif::ProgressIterator;
use log::info;

use tokio_stream::StreamExt;

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

async fn get_objects_to_delete(
    client: &Client,
    bucket_name: &str,
) -> Result<Vec<ListObjectsV2Output>, SdkError<ListObjectsV2Error, Response>> {
    info!("Calling 'list_objects_v2 to pull objects to delete");
    let paginator = client
        .list_objects_v2()
        .bucket(bucket_name)
        .into_paginator()
        .send();
    paginator
        .collect::<Result<Vec<ListObjectsV2Output>, SdkError<ListObjectsV2Error, Response>>>()
        .await
}

async fn delete_objects(
    client: &Client,
    bucket_name: &str,
    objects_to_delete: Vec<ListObjectsV2Output>,
) -> Result<usize> {
    info!("Calling 'delete_objects'");
    let mut delete_objects: Vec<ObjectIdentifier> = vec![];
    for list_output in objects_to_delete {
        for object in list_output.contents().unwrap_or_default() {
            let obj_id = ObjectIdentifier::builder()
                .set_key(Some(object.key().unwrap().to_string()))
                .build();
            delete_objects.push(obj_id);
        }
    }
    let deleted_objects_count = delete_objects.len();
    let tasks = FuturesUnordered::new();

    for chunk in delete_objects.chunks(1000).progress() {
        let task = client
            .delete_objects()
            .bucket(bucket_name)
            .delete(Delete::builder().set_objects(Some(chunk.to_vec())).build())
            .send();

        tasks.push(task);
    }
    tasks.collect::<Vec<_>>().await;

    let objects: ListObjectsV2Output = client.list_objects_v2().bucket(bucket_name).send().await?;

    match objects.key_count {
        0 => Ok(deleted_objects_count),
        _ => Err(anyhow::anyhow!(format!(
            "There were still objects left in the bucket. Failed to delete '{}' objects.",
            objects.key_count
        ))),
    }
}

pub async fn delete_bucket(
    client: &Client,
    bucket_name: &str,
    mut writer: impl std::io::Write,
) -> Result<()> {
    writeln!(writer, "Collecting objects to delete")?;
    let objects = get_objects_to_delete(client, bucket_name).await?;
    let mut counter: i32 = 0;
    for list_output in &objects {
        counter += list_output.key_count();
    }
    writeln!(writer, "Deleting {}", counter)?;
    let deleted_objects_count = delete_objects(client, bucket_name, objects).await?;
    writeln!(
        writer,
        "Successfully deleted '{}' objects.",
        deleted_objects_count
    )?;

    writeln!(writer, "Deleting the bucket '{}'.", bucket_name)?;
    client.delete_bucket().bucket(bucket_name).send().await?;
    writeln!(writer, "Bucket '{}' deleted successfully.", bucket_name)?;

    Ok(())
}
