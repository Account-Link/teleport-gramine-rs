from flask import Flask, redirect, request
from environs import Env
import os

# Read private environment vars
env = Env()
env.read_env('./private.env')
TWITTER_CONSUMER_KEY=env('TWITTER_CONSUMER_KEY')
TWITTER_CONSUMER_SECRET=env('TWITTER_CONSUMER_SECRET')
env.seal()

CALLBACK_URL="https://bobutee.soc1024.com/callback?"

app = Flask(__name__)

# Route for the index page
@app.route('/')
def index():
    page = """
    Sign up for the magic show.
<button onclick="window.open('/lfg', '_blank')">lfg</button>
<button onclick="window.open('https://bobutee.soc1024.com/approve?x_id=22847791&address=0x52668a863166eC56028Aab3aF3ed5d4F622706BB&policy=anything&nft_id=8092348023984032948', '_blank')">lfg again</button>
    """
    return page

import requests
from requests_oauthlib import OAuth1

def get_twitter_auth_url(consumer_key, consumer_secret, callback_uri):
    request_token_url = "https://api.twitter.com/oauth/request_token"
    oauth = OAuth1(consumer_key, consumer_secret, callback_uri=callback_uri)
    response = requests.post(request_token_url, auth=oauth)
    print(response.text)
    credentials = response.text.split('&')
    oauth_token = credentials[0].split('=')[1]

    return redirect(f"https://api.twitter.com/oauth/authenticate?oauth_token={oauth_token}")

# Route for the index page
@app.route('/lfg')
def lfg():
    # Generate an oauth secret
    return redirect("https://bobutee.soc1024.com/new")
    #return get_twitter_auth_url(TWITTER_CONSUMER_KEY, TWITTER_CONSUMER_SECRET, CALLBACK_URL)

# Route for the approve page
@app.route('/create')
def approve():
    #20{%20inner:%20ecdsa::Signature<Secp256k1>(CB7CD1EA2AEEB8F37B519779EADD176F6D10189D18E6472D8B92EBE916DC20F17B8093E0F15F428D8E8FE60A74211D3AFE805A69B3F257F1A871C455B2B53B9D),%20v:%20Parity(true),%20r:%2092040046076570916813934064335841288712174239762886434190714411280977071186161,%20s:%2055861657421193880590562139734829036856807898287514147071381325270955016403869%20}&success=true&id=22847791&name=Andrew+Miller&username=socrates1024&profile_image_url=https%3A%2F%2Fpbs.twimg.com%2Fprofile_images%2F1823372754520821761%2Fb2oYwBf3_normal.jpg
    x_id = request.args.get('id')
    address = "0x52668a863166eC56028Aab3aF3ed5d4F622706BB"
    policy = "anything"
    nft_id = os.urandom(8).hex()

    return redirect(f"https://bobutee.soc1024.com/approve?x_id={x_id}&address={address}&policy={policy}&nft_id={nft_id}")

# Route for the callback page
@app.route('/callback')
def callback():
    return "This is the callback page."

# Route for the done page
@app.route('/done')
def done():
    return "This is the done page."

if __name__ == '__main__':
    app.run(port=8000)
