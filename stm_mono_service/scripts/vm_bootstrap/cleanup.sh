#!/bin/bash -x
systemctl stop stm_mono_service.service
systemctl stop stm_mono_service_bootstrap.service
rm -rf /home/ubuntu/rust/logs/*
rm -f /home/ubuntu/rust/config.json
rm -f /home/ubuntu/rust/stm_mono_service
truncate -s 0 /var/log/syslog
echo "Rebuild the env with: sudo systemctl start stm_mono_service_bootstrap.service"
echo "Restart processing with: sudo systemctl start stm_mono_service.service"