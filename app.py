from flask import Flask, redirect, request
from environs import Env
import os

# Read private environment vars
env = Env()
env.read_env('./private.env')
TWITTER_CONSUMER_KEY=env('TWITTER_CONSUMER_KEY')
TWITTER_CONSUMER_SECRET=env('TWITTER_CONSUMER_SECRET')
env.seal()

app = Flask(__name__)

# Route for the index page
@app.route('/')
def index():
    nft_id = os.urandom(8).hex()
    page = f"""
    Sign up for the magic show.
<button onclick="window.open('/lfg', '_blank')">lfg</button>
<button onclick="window.open('https://bobutee.soc1024.com/approve?x_id=22847791&address=0x52668a863166eC56028Aab3aF3ed5d4F622706BB&policy=anything&nft_id={nft_id}' + , '_blank')">lfg again</button>
    """
    return page

# Route for the index page
@app.route('/lfg')
def lfg():
    # Generate an oauth secret
    return redirect("https://bobutee.soc1024.com/new")

# Route for the approve page
@app.route('/create')
def create():
    x_id = request.args.get('id')
    address = "0x52668a863166eC56028Aab3aF3ed5d4F622706BB"
    policy = "anything"
    nft_id = os.urandom(8).hex()
    return redirect(f"https://bobutee.soc1024.com/approve?x_id={x_id}&address={address}&policy={policy}&nft_id={nft_id}")

if __name__ == '__main__':
    app.run(port=8000)
