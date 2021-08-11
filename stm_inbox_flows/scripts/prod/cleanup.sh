#!/bin/bash -x
systemctl stop stm_inbox_flows.service
systemctl stop stm_inbox_flows_bootstrap.service
rm -rf /home/ubuntu/rust/logs/*
rm -f /home/ubuntu/rust/config.json
rm -f /home/ubuntu/rust/stm_inbox_flows
truncate -s 0 /var/log/syslog
echo "Rebuild the env with: sudo systemctl start stm_inbox_flows_bootstrap.service"
echo "Restart processing with: sudo systemctl start stm_inbox_flows.service"