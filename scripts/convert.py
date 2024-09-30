import csv
import json

with open("database.csv", mode="r") as csv_file:
    csv_reader = csv.reader(csv_file)
    tokens = []

    for row in csv_reader:
        token = {"token": row[2], "secret": row[3]}
        tokens.append(token)

with open("tokens.json", mode="w") as json_file:
    json.dump({"tokens": tokens}, json_file, indent=4)
