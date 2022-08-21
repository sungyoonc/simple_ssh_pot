import requests, socket, configparser, random, string

from cachetools import TTLCache

config = configparser.ConfigParser()
config.read('config.ini')

try:
  with open('config.ini') as f:
    config.read(f)
except IOError:
    raise Exception('config.ini file was not found.')

if not config['AbuseIPDB']['Key']:
  raise Exception("Missing AbuseIPDB Key from config.ini, can not continue.")

url = config['AbuseIPDB']['ReportURL']
discord_webhook_url = config['Discord']['WebhookURL']

cache = TTLCache(maxsize=50, ttl=900)

socketname = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
socketname.bind(("", 22))
socketname.listen()


print('[INFO] ListenSSH is running')

while True:
  connection, address = socketname.accept()  # address is the ip
  address = str(address).split("'")
  address = address[1]

  params = {
      'ip': str(address),
      'categories': config['AbuseIPDB']['Categories'],
      'comment': "Unauthorized connection attempt detected from IP address " + str(address) + " to port 22 " + "[" + random.choice(string.ascii_letters) + "]"
  }

  headers = {
      'Accept': 'application/json',
      'Key': config['AbuseIPDB']['Key']
  }

  message = {
      "content": "Attempted SSH Login From IP Address: " + str(address) + " Automatically Reporting To AbuseIPDB.com."
  }

  if discord_webhook_url:
    requests.post(discord_webhook_url, data=message)

  if cache.get(str(address), None) != True:
    response = requests.request(method='POST', url=url, params=params, headers=headers)

    if response.status_code == 429:
        cache[str(address)] = True
        print("IP Already reported - You must wait 15 minutes")
    else:
        cache[str(address)] = True
        print("User reported, IP: " + str(address))
        pass
  else:
    print('[INFO] Cache: ip exists in cache (TTL: 15mins)')


    
  connection.close()