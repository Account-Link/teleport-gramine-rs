import requests
import json
import os
import time

# Define the base URL for your API
BASE_URL = "https://bobutee.soc1024.com"

# Define the endpoints
REGISTER_OR_LOGIN_ENDPOINT = f"{BASE_URL}/new"
CALLBACK_ENDPOINT = f"{BASE_URL}/callback"
APPROVE_ENDPOINT = f"{BASE_URL}/approve"
MINT_ENDPOINT = f"{BASE_URL}/mint"


with open("accounts.json", "r") as file:
    accounts = json.load(file)


def read_user_infos(file_path):
    with open(file_path, "r") as file:
        user_data = json.load(file)
    return {user["username"]: user["x_id"] for user in user_data}


user_info = read_user_infos("user_infos.json")


def mint_request(session_id, username, address, policy, nft_id):
    mint_query = {
        "address": address,
        "policy": policy,
        "nft_id": nft_id,
        "x_id": user_info[username],
    }

    headers = {"Referer": APPROVE_ENDPOINT, "Content-Type": "application/json"}

    # Set up the cookies
    cookies = {"teleport_session_id": session_id}

    response = requests.post(
        MINT_ENDPOINT, headers=headers, cookies=cookies, json=mint_query
    )

    return response


nft_ids = []

for account in accounts:
    session_id = account["session_id"]
    username = account["username"]
    nft_id = os.urandom(8).hex()
    try:
        response = mint_request(
            session_id,
            username,
            "0x0b33bd59FCa63390A341ee6f608Bf5Ed1393ffcc",
            "anything!",
            nft_id,
        )
        print(username, session_id, response.text)
        nft_ids.append(nft_id)
    except Exception as e:
        print(e)
    time.sleep(1.5)

nft_ids.append(nft_id)
print(nft_ids)
