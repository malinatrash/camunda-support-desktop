#!/usr/bin/env python3
"""Seeds a local Camunda 7 engine with demo processes, waiting instances and incidents.

Usage: python3 demo/seed.py [http://localhost:8080/engine-rest]
"""
import json
import os
import random
import sys
import time
import urllib.request
import uuid

REST = (sys.argv[1] if len(sys.argv) > 1 else "http://localhost:8080/engine-rest").rstrip("/")
HERE = os.path.dirname(os.path.abspath(__file__))
NAMES = ["Alice Martin", "Bob Chen", "Carla Gomez", "Dmitry Volkov", "Emma Novak", "Farid Aziz", "Grace Kim",
         "Hiro Tanaka", "Ines Silva", "Jonas Berg", "Kira Ivanova", "Leo Rossi", "Mia Laurent", "Noah Patel"]
random.seed(7)


def call(method, path, body=None, raw=None, headers=None):
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    request = urllib.request.Request(REST + path, data=data, method=method)
    request.add_header("Content-Type", (headers or {}).get("Content-Type", "application/json"))
    with urllib.request.urlopen(request) as response:
        text = response.read()
        return json.loads(text) if text else None


def deploy(key, xml):
    boundary = uuid.uuid4().hex
    parts = [
        f'--{boundary}\r\nContent-Disposition: form-data; name="deployment-name"\r\n\r\n{key}\r\n',
        f'--{boundary}\r\nContent-Disposition: form-data; name="deploy-changed-only"\r\n\r\ntrue\r\n',
        f'--{boundary}\r\nContent-Disposition: form-data; name="{key}.bpmn"; filename="{key}.bpmn"\r\n'
        f'Content-Type: application/xml\r\n\r\n{xml}\r\n',
        f'--{boundary}--\r\n',
    ]
    call("POST", "/deployment/create", raw="".join(parts).encode(), headers={"Content-Type": f"multipart/form-data; boundary={boundary}"})


def bpmn(key):
    with open(os.path.join(HERE, "bpmn", f"{key}.bpmn")) as f:
        return f.read()


def variables(values):
    def typed(value):
        if isinstance(value, bool):
            return {"value": value, "type": "Boolean"}
        if isinstance(value, int):
            return {"value": value, "type": "Long"}
        if isinstance(value, float):
            return {"value": value, "type": "Double"}
        return {"value": value, "type": "String"}
    return {name: typed(value) for name, value in values.items()}


def start(key, business_key, values):
    return call("POST", f"/process-definition/key/{key}/start", {"businessKey": business_key, "variables": variables(values)})["id"]


def customer():
    name = random.choice(NAMES)
    return {"customerName": name, "email": name.lower().replace(" ", ".") + "@example.com"}


def work_external(topic, fail_ratio, message):
    tasks = call("POST", "/external-task/fetchAndLock", {"workerId": "demo-worker", "maxTasks": 50,
                                                          "topics": [{"topicName": topic, "lockDuration": 60000}]})
    for task in tasks:
        if random.random() < fail_ratio:
            call("POST", f"/external-task/{task['id']}/failure", {"workerId": "demo-worker", "errorMessage": message,
                                                                   "errorDetails": f"{message}\n\tat demo.Worker.handle(Worker.java:42)", "retries": 0, "retryTimeout": 0})
        elif random.random() < 0.5:
            call("POST", f"/external-task/{task['id']}/complete", {"workerId": "demo-worker"})
        else:
            call("POST", f"/external-task/{task['id']}/unlock")


def complete_tasks(key, task_key, ratio, values=None):
    for task in call("GET", f"/task?processDefinitionKey={key}&taskDefinitionKey={task_key}"):
        if random.random() < ratio:
            call("POST", f"/task/{task['id']}/complete", {"variables": variables(values or {})})


def main():
    for key in ["order-fulfillment", "customer-onboarding", "loan-application"]:
        deploy(key, bpmn(key))

    for i in range(26):
        method = random.choice(["card", "card", "invoice"])
        start("order-fulfillment", f"ORD-{10230 + i}", {**customer(), "method": method, "amount": round(random.uniform(20, 900), 2), "currency": "EUR"})
    for i in range(14):
        start("customer-onboarding", f"CUS-{5810 + i}", {**customer(), "country": random.choice(["DE", "FR", "NL", "PL", "ES"])})
    for i in range(10):
        start("loan-application", f"LOAN-{7700 + i}", {**customer(), "amount": random.choice([1500, 3000, 4800, 8000, 12000]), "disburse": i % 3 == 0, "termMonths": 24})

    loan_v2 = bpmn("loan-application").replace('name="Credit scoring"', 'name="Credit scoring v2"')
    deploy("loan-application", loan_v2)
    for i in range(8):
        start("loan-application", f"LOAN-{7710 + i}", {**customer(), "amount": random.choice([2000, 4000, 9000]), "disburse": i % 2 == 0, "termMonths": 36})

    work_external("charge-card", 0.3, "Payment gateway timeout: 504 from https://pay.example.com/charges")
    work_external("kyc-check", 0.25, "KYC provider rejected request: document image is unreadable")
    work_external("credit-score", 0.2, "Credit bureau unavailable: connection refused")
    complete_tasks("order-fulfillment", "Approve", 0.6)
    complete_tasks("customer-onboarding", "Documents", 0.8)
    complete_tasks("loan-application", "Underwriter", 0.7)

    for execution in call("GET", "/execution?activityId=Warehouse&processDefinitionKey=order-fulfillment"):
        if random.random() < 0.5:
            call("POST", "/message", {"messageName": "OrderPacked", "processInstanceId": execution["processInstanceId"]})

    time.sleep(8)
    incidents = call("GET", "/incident/count")["count"]
    instances = call("GET", "/process-instance/count")["count"]
    print(f"seeded: {instances} running instances, {incidents} incidents")


if __name__ == "__main__":
    main()
