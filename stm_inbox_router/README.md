# StackMuncher Inbox Router

This AWS Lambda function takes new stack report submissions from the inbox folder in S3, checks the payload, moves them to the member's folder in S3 and creates a new job in a Postgres table used as a queue. It does not update the member's profile.

#### Lambda deployment

Create function called `stm_inbox_router` with `stm_inbox` role, a custom runtime and customize these settings:
* env vars: see [config.rs](./src/config.rs) for the full list
* timeout: 30s
* reserved concurrency: 25
* async invocation: retry 3 times

```
cargo build --release --target x86_64-unknown-linux-gnu --package stm_inbox_router
cargo strip --target x86_64-unknown-linux-gnu
cp ./target/x86_64-unknown-linux-gnu/release/stm_inbox_router ./bootstrap && zip stm_inbox_router.zip bootstrap && rm bootstrap
aws lambda update-function-code --region us-east-1 --function-name stm_inbox_router --zip-file fileb://stm_inbox_router.zip
```

#### S3 Trigger

Configure a trigger on the inbox bucket.
* **Event name**: report_added
* **Prefix**: queue/
* **Event types**: All object create events
* **Destination**: Lambda function
* **VPC**: the same as the Postgres DB

#### Networking set up

This Lambda requires access to an RDS Postgres instance as well as to S3 via VPC. The set up involves:
* an access point on the S3 bucket - use the access point alias in place of the bucket name
* IAM policies in both, S3 access point and S3 bucket
* an endpoint of type Gateway on the VPC for connecting to S3 