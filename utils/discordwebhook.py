import configparser
import logging

import requests

logging.basicConfig(format='%(name)s - %(levelname)s - %(message)s', level=logging.INFO)

config = configparser.ConfigParser()
config.read('config.ini')

try:
  message_type = config['Discord']['Type']
except:
  logging.warn('Discord.Type is missing from config! Throwback value: "message". Please check example config.')
  message_type = 'message'

def discord_webhook(address, port, webhook_url, servername):
  message = {}

  if message_type == 'message' or not message_type :
    message = {
        "content": f'Unauthorized connection attempt detected from IP address {str(address)} to port {port} ({servername})',
    }

  if message_type == 'embed':
    message["embeds"] = [
      {
        "title" : "Unauthorized connection attempt",
        "color": "15158332",
        "description" : f'Unauthorized connection attempt detected from IP address {str(address)} to port {port}',
        "fields": [
          {
            'name': 'IP Address',
            'value': str(address)
          },
          {
            'name': 'Attacked port',
            'value': int(port)
          }
        ],
        "footer": {
          'text': f'Server: {servername}'
        }

      }
    ]

  response = requests.post(webhook_url, json=message)

  if response.status_code == 429:
    logging.warn('Ratelimited from sending webhooks')
