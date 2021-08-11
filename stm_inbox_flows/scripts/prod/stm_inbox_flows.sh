#!/bin/bash -x
/home/ubuntu/rust/stm_inbox_flows 2>&1 | rotatelogs /home/ubuntu/rust/logs/trace.%Y%m%d.%H%M.txt 20M