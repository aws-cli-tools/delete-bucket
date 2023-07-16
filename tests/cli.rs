#[allow(unused_imports)]
mod cli_tests {
    use assert_cmd::prelude::*;
    use aws_config::environment::region;
    use aws_sdk_s3::{
        operation::list_objects_v2::ListObjectsV2Output,
        primitives::ByteStream,
        types::{BucketLocationConstraint, CreateBucketConfiguration},
        Client as S3Client,
    };
    use aws_types::region::Region;
    use predicates::prelude::*;
    use rand::{distributions::Alphanumeric, Rng};
    use std::process::Command;

    #[tokio::test]
    async fn happy_flow() {
        let region = "us-east-1";
        let sdk_config = aws_config::from_env()
            .region(Region::new(region))
            .load()
            .await;
        let client = S3Client::new(&sdk_config);
        // let constraint = BucketLocationConstraint::from(region);
        let random_bucket_prefix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        let bucket_name = format!("delete-bucket-it-{}", random_bucket_prefix).to_lowercase();

        let _ = client.create_bucket().bucket(&bucket_name).send().await;

        let object_key = "your-object-key";

        let mut buffer = [0u8; 1024];
        rand::thread_rng().fill(&mut buffer[..]);

        // Upload the file
        let _ = client
            .put_object()
            .bucket(&bucket_name)
            .key(object_key)
            .body(ByteStream::from(buffer.to_vec()))
            .send()
            .await;

        let mut cmd = Command::cargo_bin("delete-bucket").unwrap();
        cmd.args(["--region", region, "--force", "--bucket", &bucket_name]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("deleted successfully."));

        let objects = client.list_objects_v2().bucket(&bucket_name).send().await;
        // Sometimes AWS deletes the bucket, but when querying for it, it might return, so I'm checking both 
        // situations.
        match objects {
            Ok(list) => assert!(list.key_count() == 0),
            Err(_) => (),
        }
    }

    #[tokio::test]
    async fn bucket_missing_show_error() {
        let mut cmd = Command::cargo_bin("delete-bucket").unwrap();
        cmd.args([
            "--region",
            "us-east-1",
            "--force",
            "--bucket",
            "bucket-is-missing.test.me",
        ]);

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("NoSuchBucket"));
    }
}
