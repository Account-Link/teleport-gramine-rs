from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
import pyotp
import requests
from eth_account import Account
import secrets
import csv
import json

BASE_URL = "https://bobutee.soc1024.com"
REGISTER_OR_LOGIN_ENDPOINT = f"{BASE_URL}/new"
CALLBACK_ENDPOINT = f"{BASE_URL}/callback"

handle_to_2fabase = {}

with open("auth2fa.csv", mode="r", newline="") as file:
    reader = csv.DictReader(file)
    for row in reader:
        handle_to_2fabase[row["handle"]] = row["2fabase"]


def gen_sk():
    priv = secrets.token_hex(32)
    private_key = "0x" + priv
    return private_key


def get_twitter_login_url(address):
    new_user_query = {"address": address, "frontend_nonce": "1"}
    response = requests.get(REGISTER_OR_LOGIN_ENDPOINT, params=new_user_query)
    return response.url


def login_to_twitter(url, username, password, totp_secret):
    driver = webdriver.Chrome()
    wait = WebDriverWait(driver, 30)

    try:
        driver.get(url)

        username_input = wait.until(
            EC.presence_of_element_located((By.ID, "username_or_email"))
        )
        username_input.send_keys(username)

        password_input = wait.until(EC.presence_of_element_located((By.ID, "password")))
        password_input.send_keys(password)

        login_button = wait.until(
            EC.presence_of_element_located(
                (
                    By.ID,
                    "allow",
                )
            )
        )
        login_button.click()

        curr_url = driver.current_url
        if "localhost" in curr_url:
            driver.get("https://bobutee.soc1024.com/")
            all_cookies = driver.get_cookies()
            cookies_dict = {}
            for cookie in all_cookies:
                cookies_dict[cookie["name"]] = cookie["value"]
            return cookies_dict["teleport_session_id"]

        totp_input = wait.until(
            EC.presence_of_element_located((By.ID, "challenge_response"))
        )
        totp = pyotp.TOTP(totp_secret)
        totp_input.send_keys(totp.now())

        totp_submit = wait.until(
            EC.presence_of_element_located((By.ID, "email_challenge_submit"))
        )
        totp_submit.click()

        curr_url = driver.current_url
        if "localhost" in curr_url:
            driver.get("https://bobutee.soc1024.com/")
            all_cookies = driver.get_cookies()
            cookies_dict = {}
            for cookie in all_cookies:
                cookies_dict[cookie["name"]] = cookie["value"]
            return cookies_dict["teleport_session_id"]

    except Exception as e:
        print("Login Failed:", e)

    finally:
        driver.quit()
        print("Login successful.")


def teleport_login(username, password):
    # priv_key = gen_sk()
    # acct = Account.from_key(priv_key)
    url = get_twitter_login_url(username)
    totp_secret = None
    if username in handle_to_2fabase:
        totp_secret = handle_to_2fabase[username]
    session_id = login_to_twitter(url, username, password, totp_secret)
    return {"session_id": session_id, "username": username}


def read_accounts_file(filename):
    credentials = []
    with open(filename, "r") as file:
        for line in file:
            parts = line.strip().split(":")
            username = parts[0]
            password = parts[1]
            credentials.append((username, password))
    return credentials


filename = "account.txt"
accounts = read_accounts_file(filename)
print(accounts)

logged_in_accounts = []

for username, password in accounts:
    login = teleport_login(username, password)
    logged_in_accounts.append(login)


with open("accounts.json", "w") as file:
    json.dump(logged_in_accounts, file)
