#!/bin/bash -x
aws s3 cp target/release/stm_inbox_flows s3://$STM_S3_BUCKET_PROD_BOOTSTRAP/apps/stm_inbox_flows