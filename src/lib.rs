use anyhow::Result;
use aws_sdk_s3::operation::list_objects_v2::{ListObjectsV2Error, ListObjectsV2Output};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use aws_sdk_s3::Client;
use aws_sdk_sts::client::customize::Response;
use aws_sdk_sts::error::SdkError;
use console::{style, Emoji};
use futures::stream::FuturesUnordered;
use indicatif::ProgressBar;
use log::info;

use tokio_stream::StreamExt;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static START_PROCESS: Emoji<'_, '_> = Emoji("🕛  ", "");
static MIDDLE_PROCESS: Emoji<'_, '_> = Emoji("🕧  ", "");
static END_PROCESS: Emoji<'_, '_> = Emoji("🕐  ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");
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
        "{} {}Collecting objects to delete",
        style("[1/5]").bold().dim(),
        LOOKING_GLASS
    )?;

    let result = get_objects_to_delete(client, bucket_name).await;

    let objects = match result {
        Ok(output) => output,
        Err(err) => {
            return Err(err.into_service_error().into());
        }
    };
    let mut counter: i32 = 0;
    for list_output in &objects {
        counter += list_output.key_count();
    }
    writeln!(
        writer,
        "{} {}Deleting {} objects ...",
        style("[2/5]").bold().dim(),
        START_PROCESS,
        counter
    )?;
    let deleted_objects_count = delete_objects(client, bucket_name, &objects).await?;
    writeln!(
        writer,
        "{} {}Successfully deleted {} objects.",
        style("[3/5]").bold().dim(),
        MIDDLE_PROCESS,
        deleted_objects_count
    )?;

    writeln!(
        writer,
        "{} {}Deleting the bucket {}.",
        style("[4/5]").bold().dim(),
        END_PROCESS,
        style(bucket_name).white()
    )?;
    client.delete_bucket().bucket(bucket_name).send().await?;
    writeln!(
        writer,
        "{} {}Bucket {} deleted successfully.",
        style("[5/5]").bold().dim(),
        SPARKLE,
        style(bucket_name).white()
    )?;

    Ok(())
}
