#!/bin/bash -x
aws s3 cp target/release/stm_mono_service s3://$STM_S3_BUCKET_PROD_BOOTSTRAP/apps/stm_mono_service