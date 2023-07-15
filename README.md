[![codecov](https://codecov.io/gh/aws-cli-tools/delete-bucket/branch/main/graph/badge.svg?token=NW4955XIZT)](https://codecov.io/gh/aws-cli-tools/delete-bucket)
[![Actions Status](https://github.com/aws-cli-tools/delete-bucket/workflows/Code%20Gating/badge.svg?branch=main)](https://github.com/aws-cli-tools/delete-bucket/workflows/Code%20Gating/badge.svg?branch=main)

# AWS S3 Bucket Deletion CLI
This command line interface (CLI) application is designed to interact with Amazon Web Services (AWS) S3 service for the purpose of deleting an existing S3 bucket. It provides an interactive way to delete a bucket and its contents securely, with optional prompt for confirmation to help prevent accidental deletion of important data.

When executed, this CLI performs the following operations:

1. Lists all the objects (files) in the given bucket.
2. Deletes all the objects retrieved from the bucket.
3. After ensuring that all objects have been deleted, it then proceeds to delete the bucket.

## Usage
To run the CLI:
```bash
delete-bucket [OPTIONS]
```

Options:

* `-h, --help` Prints help information
* `-f, --force` Do not prompt for approval.
* `-p, --profile` The AWS profile to use
* `-r, --region` The AWS region to use
* `-b, --bucket` Bucket to delete.

## Running locally
* You can always use `cargo` to manage the build and tests.
* We use [`just`](https://github.com/casey/just) as a command running.
* Use `just gate` to run all checks locally.

## Contributing
See our [CONTRIBUTION](CONTRIBUTION.md) page
