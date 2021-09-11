import requests
import socket
import configparser
import random
import string

config = configparser.ConfigParser()
config.read('config.ini')

url = config['AbuseIPDB']['ReportURL']

socketname = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

socketname.bind(("", 22))
socketname.listen()

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

  response = requests.request(
      method='POST', url=url, params=params, headers=headers)

  discord_webhook_url = config['Discord']['WebhookURL']

  message = {
      "content": "Attempted SSH Login From IP Address: " + str(address) + " Automatically Reporting To AbuseIPDB.com."
  }

  requests.post(discord_webhook_url, data=message)

  print(response.status_code)
  if response.status_code == 429:
      print("IP Already reported - You must wait 15 minutes")
  else:
      pass
  print("User reported, IP: " + str(address))
  connection.close()
