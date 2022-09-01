# Listenssh
Easily report all connection attempts on common vulnerable ports to AbuseIPDB

## Features
- AbuseIPDB reporter (with built-in ratelimits)
- Discord Webhooks (text or embed)
- IP-API integration on Discord embed webhooks


## Installation (Docker)
For easier and faster port configuration we dont recommend using Docker.

### Docker run method

```sh
docker run -v "$(pwd)"/config.ini:/listenssh/config.ini -p 22:22 -p 23:23 -p 139:139 -p 445:445 -p 3306:3306 -p 5432:5432 ghostslayer/listenssh
```

#### Docker Compose
```sh
docker-compose up -d
```

Sadly, you have to repeat the ``-p 22:22`` thing for every port you are running in config.ini.

## Installation (Manual)

Tools to install (install command below):
- [Python 3](https://www.python.org/downloads/)
- [Git](https://git-scm.com/downloads)


```sh
# Install required packages
apt install -y python3 python3-pip git

# Clone repository files
git clone https://github.com/GhostSlayer/Listenssh
cd Listenssh

# Install required python packages
pip install -r requirements.txt

# Move config file
mv config_example.ini config.ini

# Edit the config file (you may use your favorite text editor)
nano config.ini

# Start the script
python3 main.py
```

## Run in background

### Systemd
If you wish to run ListenSSH using Systemd, which we highly recommend, follow these instructions

```sh
cp systemd/listenssh.service /etc/systemd/system/listenssh.service

# Change the "WorkingDirectory" to the one where you have installed ListenSSH (unless its the root directory)
nano /etc/systemd/system/listenssh.service

systemctl daemon-reload
systemctl enable listenssh.service
systemctl start listenssh.service
```

### PM2
```sh
# Make it so PM2 restarts ListenSSH on server reboot
pm2 startup

# Start ListenSSH
pm2 start

# Save ListenSSH to PM2 so it will be restarted on reboot.
pm2 save
```