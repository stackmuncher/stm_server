#!/bin/bash -x
rm -f /home/ubuntu/rust/stm_mono_service && \
aws s3 cp s3://$STM_S3_BUCKET_PROD_BOOTSTRAP/apps/stm_mono_service /home/ubuntu/rust/stm_mono_service && \
chmod 764 /home/ubuntu/rust/stm_mono_service && \
chown ubuntu /home/ubuntu/rust/stm_mono_service 

curl -v http://169.254.169.254/latest/user-data -o /home/ubuntu/rust/config.json