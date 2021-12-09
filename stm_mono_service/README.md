# Stack Muncher Mono-Service

STM Mono-Service (STM-MS) is a monolith CLI app with multiple support and maintenance functions (flows). It is expected to run on a VM as a service or with one-off calls.

The instance is self-configuring from the *instance user data*.

1. Copy the contents of config.json into "user data" of the instance
2. `stm_mono_service_bootstrap.service` runs first and bootstraps the environment
3. `stm_mono_service.service` launches the app and starts processing as per the config

Software updates can be done by restarting the instance.

## stm_mono_service config

#### Arguments

`--flow` is optional with one of: ["www_log_reader"], optional `--log` [trace, debug, info] for logging.

The flow defaults to what is specified in the config file.

#### config.json

`config.json` contains all the info the app needs to run. It must comply with `config-schema.json` and reside in the current working folder.
The file is bootstrapped from the user instance metadata (user data) during the boot by [scripts/prod/stm_mono_service_bootstrap.sh](scripts/prod/stm_mono_service_bootstrap.sh).

## Downloader

1. Create the directory structure with `mkdirs.sh` script
- copy `stm_mono_service_bootstrap.sh` into `rust` folder and `chmod 750 stm_mono_service_bootstrap.sh` 
- run `sudo apt-get install apache2 apache2-utils` to install `logrotate`
- install `sudo apt install unzip`
- install AWS CLI from https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html
- remove AWS CLI installation files
- check if the VM has access to all the other AWS resources it needs (IAM)


2. Create `sudo nano /etc/systemd/system/stm_mono_service_bootstrap.service` with
```
[Unit]
Description=StackMuncher App Bootstrapper
After=network.target

[Service]
Type=oneshot
Environment=STM_S3_BUCKET_PROD_BOOTSTRAP=stm-apps-...
ExecStart=/bin/bash -ce 'exec /home/ubuntu/rust/stm_mono_service_bootstrap.sh'
RemainAfterExit=true
StandardOutput=journal

[Install]
WantedBy=multi-user.target
```
Set `STM_S3_BUCKET_PROD_BOOTSTRAP` value to the bucket name with the executable. There is no need to create this env var for `ubuntu` or any other user.

1. Create `sudo nano /etc/systemd/system/stm_mono_service.service` with the following content:

```
[Unit]
Description=StackMuncher Mono-Service
After=network.target
StartLimitIntervalSec=0
After=stm_mono_service_bootstrap.service

[Service]
Type=simple
Restart=always
RestartSec=10
ExecStart=/bin/bash -ce 'exec /home/ubuntu/rust/stm_mono_service.sh'
WorkingDirectory=/home/ubuntu/rust
User=ubuntu
Nice=5
Environment=RUST_BACKTRACE=1

[Install]
WantedBy=multi-user.target
```
and `chmod 750 stm_mono_service.sh`

3. Set user password for `ubuntu` with `sudo passwd ubuntu`. It doesn't matter the pwd is set to as long as its set.

4. Run 
 - `sudo systemctl enable stm_mono_service_bootstrap.service`
 - `sudo systemctl enable stm_mono_service.service`
 - `sudo systemctl start stm_mono_service_bootstrap.service`

Check that `stm_mono_service` and `config.json` are present in the working directory. Reboot or start the downloader manually.

Refs:
- https://medium.com/@benmorel/creating-a-linux-service-with-systemd-611b5c8b91d6
- https://stackoverflow.com/a/46164095/6923661
- https://www.man7.org/linux/man-pages/man5/systemd.service.5.html
- https://www.man7.org/linux/man-pages/man5/systemd.exec.5.html
- https://www.man7.org/linux/man-pages/man5/systemd.unit.5.html
- https://www.man7.org/linux/man-pages/man1/systemd.1.html

#### Starting and stopping after config

- `sudo systemctl start stm_mono_service.service`
- `sudo systemctl stop stm_mono_service.service`
- `sudo systemctl daemon-reload`
- `sudo bash cleanup.sh`

#### Launching it manually as an app can be done with:
```bash
cd rust
./stm_mono_service 2>&1 | rotatelogs trace.%Y%m%d.%H%M.txt 20M &
disown -h %1
```

#### Log maintenance

The log files clog up the drive and have to be removed using a cron job (`sudo crontab -e`). This cron job helps with long-lived instances with low processing volume.
See [scripts/prod/crontab.bak](scripts/prod/crontab.bak) for details.

## FLows

### Search results log

`--flow www_log_reader` merges web logs produced by CloudFront with internal search-to-results logging.

The web logs contain the IP of the requestor and the HTTP headers. Only the IP address reaches the Lambda that fulfills that request, so it's impossible to say if it was from a bot or a human. This flow reconciles the www-log info with the search-log and saves results that came from humans (not known bots, really) in ElasticSearch.


## Updating an AMI after changes

1. Run `sudo /home/ubuntu/rust/cleanup.sh`
2. Create a new AMI with reboot
3. The machine should bootstrap itself during the reboot and resume with the same flow
4. Edit the launch template to use the latest AMI 


## Instance sizes

All instances are stateless and can run as _spot_ or _persistent spot_.

* **www_log_reader**: nano
