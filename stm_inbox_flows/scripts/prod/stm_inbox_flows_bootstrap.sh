#!/bin/bash -x
rm -f /home/ubuntu/rust/stm_inbox_flows && \
aws s3 cp s3://$STM_S3_BUCKET_PROD_BOOTSTRAP/apps/stm_inbox_flows /home/ubuntu/rust/stm_inbox_flows && \
chmod 764 /home/ubuntu/rust/stm_inbox_flows && \
chown ubuntu /home/ubuntu/rust/stm_inbox_flows 

curl -v http://169.254.169.254/latest/user-data -o /home/ubuntu/rust/config.json