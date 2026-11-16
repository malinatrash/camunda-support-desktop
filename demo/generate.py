#!/usr/bin/env python3
"""Generates the demo BPMN models used by demo/seed.sh (with diagram layout)."""
import os

SIZES = {"start": (36, 36), "end": (36, 36), "xor": (50, 50), "task": (110, 80)}
TAGS = {"start": "startEvent", "end": "endEvent", "xor": "exclusiveGateway"}


def model(key, name, nodes, flows):
    """nodes: (id, kind, label, x, y, attrs); flows: (source, target, condition)."""
    out = []
    shapes = []
    for node_id, kind, label, x, y, attrs in nodes:
        tag = attrs.pop("tag", None) or TAGS.get(kind, "task")
        extra = "".join(f' {k}="{v}"' for k, v in attrs.items())
        out.append(f'    <bpmn:{tag} id="{node_id}" name="{label}"{extra} />')
        w, h = SIZES[kind]
        shapes.append((node_id, x, y, w, h, kind))
    centers = {s[0]: s for s in shapes}
    edges = []
    for i, (src, dst, cond) in enumerate(flows):
        fid = f"Flow_{i}"
        body = f'<bpmn:conditionExpression xsi:type="bpmn:tFormalExpression">{cond}</bpmn:conditionExpression>' if cond else ""
        out.append(f'    <bpmn:sequenceFlow id="{fid}" sourceRef="{src}" targetRef="{dst}">{body}</bpmn:sequenceFlow>')
        _, sx, sy, sw, sh, _ = centers[src]
        _, tx, ty, tw, th, _ = centers[dst]
        a = (sx + sw, sy + sh / 2)
        b = (tx, ty + th / 2)
        if abs(a[1] - b[1]) < 1:
            pts = [a, b]
        elif sy + sh / 2 < ty + th / 2:
            pts = [(sx + sw / 2, sy + sh), (sx + sw / 2, ty + th / 2), b]
        else:
            pts = [a, (tx + tw / 2, sy + sh / 2), (tx + tw / 2, ty + th)]
        edges.append((fid, pts))
    di = []
    for node_id, x, y, w, h, kind in shapes:
        di.append(f'      <bpmndi:BPMNShape id="{node_id}_di" bpmnElement="{node_id}"><dc:Bounds x="{x}" y="{y}" width="{w}" height="{h}" /></bpmndi:BPMNShape>')
    for fid, pts in edges:
        wps = "".join(f'<di:waypoint x="{int(px)}" y="{int(py)}" />' for px, py in pts)
        di.append(f'      <bpmndi:BPMNEdge id="{fid}_di" bpmnElement="{fid}">{wps}</bpmndi:BPMNEdge>')
    return f'''<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" xmlns:dc="http://www.omg.org/spec/DD/20100524/DC" xmlns:di="http://www.omg.org/spec/DD/20100524/DI" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:camunda="http://camunda.org/schema/1.0/bpmn" id="Definitions_{key}" targetNamespace="http://camunda.org/examples">
  <bpmn:process id="{key}" name="{name}" isExecutable="true" camunda:historyTimeToLive="30">
{chr(10).join(out)}
  </bpmn:process>
  <bpmndi:BPMNDiagram id="Diagram_{key}">
    <bpmndi:BPMNPlane id="Plane_{key}" bpmnElement="{key}">
{chr(10).join(di)}
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</bpmn:definitions>
'''


def ext(topic):
    return {"camunda:type": "external", "camunda:topic": topic, "tag": "serviceTask"}


def expr(value, async_before=False):
    attrs = {"camunda:expression": value, "tag": "serviceTask"}
    if async_before:
        attrs["camunda:asyncBefore"] = "true"
    return attrs


Y = 100
MODELS = {
    "order-fulfillment": model("order-fulfillment", "Order fulfillment", [
        ("Start", "start", "Order received", 100, Y + 22, {}),
        ("Validate", "task", "Validate order", 180, Y, expr("${true}")),
        ("Method", "xor", "Payment method?", 340, Y + 15, {}),
        ("Charge", "task", "Charge card", 440, Y, ext("charge-card")),
        ("Approve", "task", "Approve invoice", 440, Y + 140, {"tag": "userTask"}),
        ("Join", "xor", "", 600, Y + 15, {}),
        ("Warehouse", "task", "Wait for warehouse", 700, Y, {"tag": "receiveTask", "messageRef": "Msg_Packed"}),
        ("Ship", "task", "Ship order", 860, Y, expr("${shipping.dispatch(execution)}", True)),
        ("End", "end", "Order shipped", 1020, Y + 22, {}),
    ], [
        ("Start", "Validate", None), ("Validate", "Method", None),
        ("Method", "Charge", "${method == 'card'}"), ("Method", "Approve", "${method != 'card'}"),
        ("Charge", "Join", None), ("Approve", "Join", None), ("Join", "Warehouse", None),
        ("Warehouse", "Ship", None), ("Ship", "End", None),
    ]),
    "customer-onboarding": model("customer-onboarding", "Customer onboarding", [
        ("Start", "start", "Sign-up submitted", 100, Y + 22, {}),
        ("Documents", "task", "Collect documents", 180, Y, {"tag": "userTask"}),
        ("Kyc", "task", "KYC check", 340, Y, ext("kyc-check")),
        ("Review", "task", "Manual review", 500, Y, {"tag": "userTask"}),
        ("Account", "task", "Create account", 660, Y, expr("${true}")),
        ("End", "end", "Customer active", 820, Y + 22, {}),
    ], [
        ("Start", "Documents", None), ("Documents", "Kyc", None), ("Kyc", "Review", None),
        ("Review", "Account", None), ("Account", "End", None),
    ]),
    "loan-application": model("loan-application", "Loan application", [
        ("Start", "start", "Application received", 100, Y + 22, {}),
        ("Score", "task", "Credit scoring", 180, Y, ext("credit-score")),
        ("Risk", "xor", "Risk level?", 340, Y + 15, {}),
        ("Underwriter", "task", "Underwriter decision", 440, Y + 140, {"tag": "userTask"}),
        ("Join", "xor", "", 600, Y + 15, {}),
        ("Disburse", "task", "Disburse funds", 700, Y, expr("${disburse ? payments.send(execution) : true}", True)),
        ("End", "end", "Loan issued", 860, Y + 22, {}),
    ], [
        ("Start", "Score", None), ("Score", "Risk", None),
        ("Risk", "Join", "${amount &lt;= 5000}"), ("Risk", "Underwriter", "${amount &gt; 5000}"),
        ("Underwriter", "Join", None), ("Join", "Disburse", None), ("Disburse", "End", None),
    ]),
}

here = os.path.dirname(os.path.abspath(__file__))
for key, xml in MODELS.items():
    if key == "order-fulfillment":
        xml = xml.replace('<bpmn:process', '<bpmn:message id="Msg_Packed" name="OrderPacked" />\n  <bpmn:process', 1)
    with open(os.path.join(here, "bpmn", f"{key}.bpmn"), "w") as f:
        f.write(xml)
print("ok")
