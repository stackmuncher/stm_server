# Stack Muncher Inbox Flows

#### This app is a collection of handlers (flows) running as a background process on a VM.

The VM instance is self-configuring from the EC2 *instance user data* with this bootstrap sequence:

1. Copy the contents of config.json into "user data" of the instance
2. `stm_inbox_flows_bootstrap.service` will run first and bootstrap the environment
3. `stm_inbox_flows.service` will launch the app and start processing as per the config

Software updates can be done by restarting the instance to invoke the bootstrapping script and re-read the config from VM's user data.

## stm_inbox_flows config

#### Build and deploy
```shell
cargo build --release --target x86_64-unknown-linux-gnu --package stm_inbox_flows
cargo strip --target x86_64-unknown-linux-gnu
aws s3 cp target/x86_64-unknown-linux-gnu/release/stm_inbox_flows s3://$STM_S3_BUCKET_PROD_BOOTSTRAP/apps/stm_inbox_flows
```
Restart the instance - it will run its bootstrap script which downloads the latest binary from the above S3 location.

#### Arguments

`-flow` is optional with one of: ["dev_queue",], optional `-l` [trace, debug, info] for logging.

The flow defaults to what is specified in the config file.

#### config.json

`config.json` contains all the info the app needs to run. It must comply with `config_schema.json` and reside in the current working folder.
The file is bootstrapped from the user instance metadata (user data) during the boot by [scripts/prod/stm_inbox_flows_bootstrap.sh](scripts/prod/stm_inbox_flows_bootstrap.sh).

## VM set up

1. Install pre-requisites
- **rotatelogs**: `sudo apt-get install apache2 apache2-utils`
- **unzip**: `sudo apt install unzip`
- **AWS CLI**: `curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip" && unzip awscliv2.zip && sudo ./aws/install`

2. Create the directory structure with `mkdirs.sh` script
3. `cd rust` and copy or manually create the following:
- copy `stm_inbox_flows_bootstrap.sh`, `stm_inbox_flows.sh`, `cleanup.sh` into `rust` folder
- run `chmod u=rwx,g=rx,o=r /home/ubuntu/rust/*.sh` to make the shell scripts runnable


4. Create `sudo nano /etc/systemd/system/stm_inbox_flows_bootstrap.service` with
```
[Unit]
Description=StackMuncher App Bootstrapper
After=network.target

[Service]
Type=oneshot
Environment=STM_S3_BUCKET_PROD_BOOTSTRAP=stm-apps-...
ExecStart=/bin/bash -ce 'exec /home/ubuntu/rust/stm_inbox_flows_bootstrap.sh'
RemainAfterExit=true
StandardOutput=journal

[Install]
WantedBy=multi-user.target
```
Set `STM_S3_BUCKET_PROD_BOOTSTRAP` value to the bucket name with the executable.

5. Create `sudo nano /etc/systemd/system/stm_inbox_flows.service` with the following content:

```
[Unit]
Description=StackMuncher Inbox Flows
After=network.target
StartLimitIntervalSec=0
After=stm_inbox_flows_bootstrap.service

[Service]
Type=simple
Restart=always
RestartSec=10
ExecStart=/bin/bash -ce 'exec /home/ubuntu/rust/stm_inbox_flows.sh'
WorkingDirectory=/home/ubuntu/rust
User=ubuntu
Nice=5
Environment=RUST_BACKTRACE=1

[Install]
WantedBy=multi-user.target
```

6. Set user password for `ubuntu` with `sudo passwd ubuntu`. The password is not used by any of the scripts, but it has to be set in order for the service to function.

7. Run 
 - `sudo systemctl enable stm_inbox_flows_bootstrap.service`
 - `sudo systemctl enable stm_inbox_flows.service`
 - `sudo systemctl start stm_inbox_flows_bootstrap.service`

Check that `stm_inbox_flows` and `config.json` are present in the working directory. Reboot or start the stm_inbox_flows app manually.

Refs:
- https://medium.com/@benmorel/creating-a-linux-service-with-systemd-611b5c8b91d6
- https://stackoverflow.com/a/46164095/6923661
- https://www.man7.org/linux/man-pages/man5/systemd.service.5.html
- https://www.man7.org/linux/man-pages/man5/systemd.exec.5.html
- https://www.man7.org/linux/man-pages/man5/systemd.unit.5.html
- https://www.man7.org/linux/man-pages/man1/systemd.1.html

#### Starting and stopping after config

- `sudo systemctl start stm_inbox_flows.service`
- `sudo systemctl stop stm_inbox_flows.service`
- `sudo systemctl daemon-reload`
- `sudo bash cleanup.sh`

#### Launching the app manually can be done with:
```bash
cd rust
./stm_inbox_flows 2>&1 | rotatelogs trace.%Y%m%d.%H%M.txt 20M &
disown -h %1
```

#### Log maintenance

The log files clog up the drive and have to be removed using a cron job (`sudo crontab -e`). This cron job helps with long-lived instances with low processing volume.
See [scripts/prod/crontab.bak](scripts/prod/crontab.bak) for details.

## Updating an AMI after changes

1. Run `sudo /home/ubuntu/rust/cleanup.sh`
2. Create a new AMI with reboot
3. The machine should bootstrap itself during the reboot and resume with the same flow
4. Edit the launch template to use the latest AMI 


## FLows

### Updating dev profiles from submitted reports

`-flow dev_queue` processes reports after *stm_inbox* and *stm_inbox_router* steps. It loads the contents of the reports and combines them into a single dev profile. Public profile details such name and contact are displayed exactly as they are in the very last report. Dev profiles are saved in ElasticSearch and S3.
