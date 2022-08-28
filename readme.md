# Listenssh
ORIGIANL CODE BY [pccs](https://pccs.uk), this is a modified one by me. I have pretty small python experience so hope everything is working fine

## Installation (Docker)

### Docker run method

``docker run -v "$(pwd)"/config.ini:/listenssh/config.ini -p 22:22 -p 23:23 -p 139:139 -p 445:445 -p 3306:3306 -p 5432:5432 listenssh``

You have to repeat the ``-p 22:22`` thing for every port you are running in config.ini.
### Docker compose

You might have to customize the docker-compose.yml file for your needs.

### Get all files from GitHub
```
git clone https://github.com/GhostSlayer/Listenssh
```

### Install dependencies
```
pip install -r requirements.txt
```

### Change example config file to the primary one
```
mv config_example.ini config.ini
```

### Update config
All configs are required at the moment.

```
nano config.ini
```

### Start script
##### Using Python script
```
python3 main.py
```

##### Using Process Manager 2
```
pm2 start
```