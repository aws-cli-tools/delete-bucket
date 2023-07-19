#[allow(unused_imports)]
mod cli_tests {
    use assert_cmd::prelude::*;
    use aws_config::environment::region;
    use aws_sdk_s3::{
        operation::list_objects_v2::ListObjectsV2Output,
        primitives::ByteStream,
        types::{
            BucketLocationConstraint, BucketVersioningStatus, CreateBucketConfiguration,
            VersioningConfiguration,
        },
        Client as S3Client,
    };
    use aws_types::region::Region;
    use predicates::prelude::*;
    use rand::{distributions::Alphanumeric, Rng};
    use std::process::Command;

    const REGION: &str = "us-east-1";
    async fn create_tmp_bucket(client: &S3Client, bucket_name: &str) {
        let _ = client.create_bucket().bucket(bucket_name).send().await;

        let versioning_config = VersioningConfiguration::builder()
            .set_status(Some(BucketVersioningStatus::Enabled))
            .build();

        let _ = client
            .put_bucket_versioning()
            .bucket(bucket_name)
            .versioning_configuration(versioning_config)
            .send()
            .await;

        let object_key = "your-object-key";

        let mut buffer = [0u8; 1024];
        rand::thread_rng().fill(&mut buffer[..]);

        // Upload the file
        let _ = client
            .put_object()
            .bucket(bucket_name)
            .key(object_key)
            .body(ByteStream::from(buffer.to_vec()))
            .send()
            .await;
        // Reupload the file
        let _ = client
            .put_object()
            .bucket(bucket_name)
            .key(object_key)
            .body(ByteStream::from(buffer.to_vec()))
            .send()
            .await;

        // Delete one of the files to create a Delete Marker
        let object_key = "your-object-key-delete";

        let mut buffer = [0u8; 1024];
        rand::thread_rng().fill(&mut buffer[..]);

        let _ = client
            .put_object()
            .bucket(bucket_name)
            .key(object_key)
            .body(ByteStream::from(buffer.to_vec()))
            .send()
            .await;

        let _ = client
            .delete_object()
            .bucket(bucket_name)
            .key(object_key)
            .send()
            .await;
    }
    #[tokio::test]
    async fn happy_flow() {
        let random_bucket_prefix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        let bucket_name = format!("delete-bucket-it-{}", random_bucket_prefix).to_lowercase();
        let bucket_name2 = format!("delete-bucket2-it-{}", random_bucket_prefix).to_lowercase();

        let sdk_config = aws_config::from_env()
            .region(Region::new(REGION))
            .load()
            .await;
        let client = S3Client::new(&sdk_config);

        create_tmp_bucket(&client, &bucket_name).await;
        create_tmp_bucket(&client, &bucket_name2).await;

        let mut cmd = Command::cargo_bin("delete-bucket").unwrap();
        cmd.args([
            "--region",
            REGION,
            "--force",
            "--buckets",
            &bucket_name,
            &bucket_name2,
        ]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("deleted successfully."));

        let objects = client.list_objects_v2().bucket(&bucket_name).send().await;
        // Sometimes AWS deletes the bucket, but when querying for it, it might return, so I'm checking both
        // situations.
        if let Ok(list) = objects {
            assert!(list.key_count() == 0)
        }
    }

    #[tokio::test]
    async fn bucket_missing_show_error() {
        let mut cmd = Command::cargo_bin("delete-bucket").unwrap();
        cmd.args([
            "--region",
            "us-east-1",
            "--force",
            "--buckets",
            "bucket-is-missing.test.me",
        ]);

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("NoSuchBucket"));
    }
}
