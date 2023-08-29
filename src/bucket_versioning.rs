use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
};

use aws_sdk_s3::{
    operation::{
        delete_object::{DeleteObjectError, DeleteObjectOutput},
        list_object_versions::ListObjectVersionsError,
    },
    types::{BucketVersioningStatus, VersioningConfiguration},
    Client,
};
use aws_sdk_sts::error::SdkError;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use futures::{stream::FuturesUnordered, StreamExt};
use governor::{Quota, RateLimiter};
use indicatif::ProgressBar;
use log::info;

type DeleteObjectResult = Result<DeleteObjectOutput, SdkError<DeleteObjectError, HttpResponse>>;
type SharedResults = Arc<Mutex<Vec<DeleteObjectResult>>>;

pub async fn disable_versioning(client: &Client, bucket_name: &str) -> bool {
    if let Ok(has_versioning) = client
        .get_bucket_versioning()
        .bucket(bucket_name)
        .send()
        .await
    {
        let status = match has_versioning.status() {
            Some(val) => *val == BucketVersioningStatus::Enabled,
            None => false,
        };
        if status {
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

            true
        } else {
            false
        }
    } else {
        false
    }
}

pub async fn delete_versioned_objects(
    client: &Client,
    bucket_name: &str,
) -> Result<(usize, usize), SdkError<ListObjectVersionsError>> {
    info!("Calling 'delete_versioned_objects'");
    let mut all_versions: Vec<(String, String)> = Vec::new();
    let mut next_key_marker: Option<String> = None;
    let mut next_version_id_marker: Option<String> = None;

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
    let rate = Quota::per_second(NonZeroU32::new(500).unwrap());
    let rate_limiter = Arc::new(RateLimiter::direct(rate));

    for item in all_versions {
        let rate_limiter = rate_limiter.clone();
        let task = client
            .delete_object()
            .bucket(bucket_name)
            .version_id(item.0)
            .key(item.1)
            .send();

        let local_pb = pb.clone();
        let wrapped_task = async move {
            rate_limiter.until_ready().await;
            let result = task.await;
            local_pb.inc(1);
            result
        };

        tasks.push(wrapped_task);
    }
    let results: SharedResults = Arc::new(Mutex::new(Vec::new()));

    tasks
        .for_each_concurrent(2, |result| async {
            let mut locked_results = results.lock().unwrap();
            locked_results.push(result);

            if let Err(e) = &locked_results.last().unwrap() {
                info!("Error deleting: {:?}", e);
            }
        })
        .await;

    let successful_count = results
        .lock()
        .unwrap()
        .iter()
        .filter(|&result| result.is_ok())
        .count();
    let failed_count = deleted_objects_count - successful_count;

    Ok((successful_count, failed_count))
}
