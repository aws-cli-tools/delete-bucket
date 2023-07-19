use anyhow::Result;
use aws_sdk_s3::operation::list_object_versions::ListObjectVersionsError;
use aws_sdk_s3::operation::list_objects_v2::{ListObjectsV2Error, ListObjectsV2Output};
use aws_sdk_s3::types::{
    BucketVersioningStatus, Delete, ObjectIdentifier, VersioningConfiguration,
};
use aws_sdk_s3::Client;
use aws_sdk_sts::client::customize::Response;
use aws_sdk_sts::error::SdkError;
use console::style;
use futures::stream::FuturesUnordered;
use indicatif::ProgressBar;
use log::info;

use tokio_stream::StreamExt;

const CHUNK_SIZE: usize = 1000;

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

async fn delete_versioned_objects(
    client: &Client,
    bucket_name: &str,
) -> Result<(usize, usize), SdkError<ListObjectVersionsError>> {
    info!("Calling 'delete_versioned_objects'");
    let mut all_versions: Vec<(String, String)> = Vec::new();
    let mut next_key_marker: Option<String> = None;
    let mut next_version_id_marker: Option<String> = None;

    // Suspend object versioning
    let versioning_config = VersioningConfiguration::builder()
        .set_status(Some(BucketVersioningStatus::Suspended))
        .build();

    let _ = client
        .put_bucket_versioning()
        .bucket(bucket_name)
        .versioning_configuration(versioning_config)
        .send()
        .await;

    // No paginator :-(
    loop {
        let mut request_builder = client.list_object_versions().bucket(bucket_name);

        if let Some(marker) = next_key_marker {
            request_builder = request_builder.key_marker(marker);
        }

        if let Some(marker) = next_version_id_marker {
            request_builder = request_builder.version_id_marker(marker);
        }

        let result = request_builder.send().await?;

        if let Some(versions) = result.versions() {
            let tuples: Vec<(String, String)> = versions
                .iter()
                .filter_map(|version| {
                    let version_id = version.version_id()?;
                    let key = version.key()?;
                    Some((version_id.to_string(), key.to_string()))
                })
                .collect();

            all_versions.extend_from_slice(&tuples);
        }
        if let Some(versions) = result.delete_markers() {
            let tuples: Vec<(String, String)> = versions
                .iter()
                .filter_map(|version| {
                    let version_id = version.version_id()?;
                    let key = version.key()?;
                    Some((version_id.to_string(), key.to_string()))
                })
                .collect();

            all_versions.extend_from_slice(&tuples);
        }

        if result.is_truncated() {
            next_key_marker = result.next_key_marker().map(|s| s.to_string());
            next_version_id_marker = result.next_version_id_marker().map(|s| s.to_string());
        } else {
            break;
        }
    }

    let deleted_objects_count = all_versions.len();
    let tasks = FuturesUnordered::new();
    let pb = ProgressBar::new(deleted_objects_count as u64);

    for item in all_versions {
        let task = client
            .delete_object()
            .bucket(bucket_name)
            .version_id(item.0)
            .key(item.1)
            .send();

        let local_pb = pb.clone();
        let wrapped_task = async move {
            let result = task.await;
            local_pb.inc(1);
            result
        };

        tasks.push(wrapped_task);
    }

    let results = tasks.collect::<Vec<_>>().await;

    let successful_count = results.iter().filter(|&result| result.is_ok()).count();
    let failed_count = deleted_objects_count - successful_count;

    Ok((successful_count, failed_count))
}

async fn delete_objects(
    client: &Client,
    bucket_name: &str,
    objects_to_delete: &[ListObjectsV2Output],
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
    let num_tasks = delete_objects.chunks(CHUNK_SIZE).len();
    let pb = ProgressBar::new(num_tasks as u64);

    for chunk in delete_objects.chunks(CHUNK_SIZE) {
        let task = client
            .delete_objects()
            .bucket(bucket_name)
            .delete(Delete::builder().set_objects(Some(chunk.to_vec())).build())
            .send();

        let local_pb = pb.clone();
        let wrapped_task = async move {
            let result = task.await;
            local_pb.inc(1);
            result
        };

        tasks.push(wrapped_task);
    }
    tasks.collect::<Vec<_>>().await;

    let objects: ListObjectsV2Output = client.list_objects_v2().bucket(bucket_name).send().await?;
    if objects.key_count == 0 {
        Ok(deleted_objects_count)
    } else {
        Err(anyhow::anyhow!(format!(
            "There were still objects left in the bucket. Failed to delete '{}' objects.",
            objects.key_count
        )))
    }
}

pub async fn delete_bucket(
    client: &Client,
    bucket_name: &str,
    mut writer: impl std::io::Write,
) -> Result<()> {
    writeln!(
        writer,
        "{} Disabling Bucket Versioning and deleting versioned objects",
        style("[1/6]").bold().dim(),
    )?;

    let (successful_count, failed_count) = match delete_versioned_objects(client, bucket_name).await
    {
        Ok((successful_count, failed_count)) => (successful_count, failed_count),
        Err(err) => {
            let service_error = err.into_service_error();
            info!("Call failed {:?}", service_error);
            return Err(anyhow::anyhow!("{}", service_error.meta().code().unwrap()));
        }
    };
    let failed_text = if failed_count > 0 {
        format!("Failed deleting {} versioned objects", failed_count)
    } else {
        String::from("")
    };
    writeln!(
        writer,
        "{} Successfully deleted {} versioned objects. {}",
        style("[2/6]").bold().dim(),
        successful_count,
        failed_text
    )?;

    if failed_count > 0 {
        info!("Failed deleting all objects.");
        return Err(anyhow::anyhow!("Failed deleting all objects."));
    }

    writeln!(
        writer,
        "{} Collecting non-versioned objects to delete",
        style("[3/6]").bold().dim(),
    )?;

    let result = get_objects_to_delete(client, bucket_name).await;

    let objects = match result {
        Ok(output) => output,
        Err(err) => {
            let service_error = err.into_service_error();
            info!("Call failed {:?}", service_error);
            return Err(anyhow::anyhow!("{}", service_error.meta().code().unwrap()));
        }
    };
    let mut counter: i32 = 0;
    for list_output in &objects {
        counter += list_output.key_count();
    }
    writeln!(
        writer,
        "{} Deleting {} objects ...",
        style("[3/6]").bold().dim(),
        counter
    )?;
    let deleted_objects_count = delete_objects(client, bucket_name, &objects).await?;
    writeln!(
        writer,
        "{} Successfully deleted {} objects.",
        style("[4/6]").bold().dim(),
        deleted_objects_count
    )?;

    writeln!(
        writer,
        "{} Deleting the bucket {}.",
        style("[5/6]").bold().dim(),
        style(bucket_name).white()
    )?;
    if let Err(err) = client.delete_bucket().bucket(bucket_name).send().await {
        let service_error = err.into_service_error();
        info!("Call failed {:?}", service_error);
        return Err(anyhow::anyhow!("{}", service_error.meta().code().unwrap()));
    }

    writeln!(
        writer,
        "{} Bucket {} deleted successfully.",
        style("[6/6]").bold().dim(),
        style(bucket_name).white()
    )?;

    Ok(())
}
