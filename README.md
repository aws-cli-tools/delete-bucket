[![Actions Status](https://github.com/aws-cli-tools/delete-bucket/workflows/Code%20Gating/badge.svg?branch=main)](https://github.com/aws-cli-tools/delete-bucket/workflows/Code%20Gating/badge.svg?branch=main)
[![Actions Status](https://img.shields.io/badge/built%20with%20rust-red?logo=rust)](https://img.shields.io/badge/built%20with%20rust-red?logo=rust)

# AWS S3 Bucket Deletion CLI

<p align="center">
  <img src="https://github.com/aws-cli-tools/delete-bucket/assets/110536677/1433c22b-e555-4722-adb4-29ea5d2bf7f8" alt="A comics painting show an AWS S3 bucket being cut to pieces" width="256" height="256">
</p>

This CLI application is designed to interact with AWS S3 service for the purpose of deleting an existing S3 bucket. It provides an interactive way to delete a bucket and its contents securely, with optional prompt for confirmation to help prevent accidental deletion of important data.

When executed, this CLI performs the following operations:

1. Detect whether the bucket has Object Versioning enabled and disable it if it does.
2. Lists all the objects in the given bucket.
3. Deletes all the objects retrieved from the bucket.
4. After ensuring that all objects have been deleted, it then proceeds to delete the bucket.

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
* `-b, --buckets` Buckets to delete.

## Speed comparison
Deleting 10K files in a single bucket on `c6i.2xlarge` machine:
* `delete-bucket` - 6.234s
* `aws s3 rm s3://bucket-name --recursive` [AWS Docs](https://docs.aws.amazon.com/AmazonS3/latest/userguide/empty-bucket.html) - 25.086s

That's fast! 💨

## Installation

There are two main methods for installing this tool:

### Method 1: Download binaries

You can download the pre-compiled binaries directly from the GitHub releases page. Choose the correct binary depending on your operating system.

Visit the [releases page](https://github.com/aws-cli-tools/delete-bucket/releases) to download the appropriate binary.
For example (make sure to use the latest version):
```bash
wget https://github.com/aws-cli-tools/delete-bucket/releases/download/v0.1.5/delete-bucket-v0.1.5-x86_64-linux.tar.xz
tar -xf delete-bucket-v0.1.5-x86_64-linux.tar.xz
```

### Method 2: Using Homebrew (for macOS users)

If you are a macOS user and have [Homebrew](https://brew.sh/) installed, you can install this tool using the following commands:

```shell
brew tap aws-cli-tools/aws-cli-tools
brew install delete-bucket
```
## Running locally
* You can always use `cargo` to manage the build and tests.
* We use [`just`](https://github.com/casey/just) as a command running.
* Use `just gate` to run all checks locally.

## Contributing
See our [CONTRIBUTION](CONTRIBUTION.md) page
