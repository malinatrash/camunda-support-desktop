# Product specification

## 1. Overview

**Camunda Support** is a native desktop console for engineers who operate Camunda 7. It replaces hand-edited curl commands with a guided interface: find a case, understand why a process is stuck, and run an allowed recovery operation.

It is not a generic REST client and not a BPMN editor. The app exposes a limited catalog of support operations and always shows the environment, the business key, the process instance and the expected result of an action.

### Goals

1. Shorten the path from a business key to the reason a process is stuck.
2. Reduce the risk of running an operation in the wrong environment or on the wrong instance.
3. Show incidents, jobs, external tasks, variables and process versions in one place.
4. Replace hand-written modification JSON with a step-by-step teleport (token move).
5. Keep a local history of actions, even when Camunda operation history is incomplete.

### Out of scope

- editing and deploying BPMN;
- arbitrary Camunda REST requests;
- bulk production operations in early releases;
- replacing Camunda Cockpit for platform administration;
- a separate backend, proxy or server-side database.

## 2. Users

### Support viewer

Configures connections, searches for instances, and inspects incidents, variables and history. Can open a process in Cockpit.

### Support operator

Can also retry jobs and external tasks, suspend and activate a process, correlate allowed messages and run a step-by-step teleport.

In the client-only version, real permissions are defined by the Camunda credentials. Local app roles protect the interface but are not a full security boundary.

## 3. Screen hierarchy

```text
App
├── Connection picker
│   ├── Production
│   ├── Staging
│   └── Local
│
├── Overview
│   ├── Connection state
│   ├── Latest process versions
│   ├── Current instances and incidents
│   ├── Statistics for today / yesterday / period
│   └── List and card previews
│
├── Processes
│   ├── Search by business key / ID / variable
│   ├── Process definitions
│   │   └── Versions
│   │       ├── Active instances
│   │       └── Instances with incidents
│   └── Process instance
│       ├── General info
│       ├── Current activities
│       ├── Incidents
│       ├── External tasks
│       ├── Jobs
│       ├── Variables
│       ├── History
│       ├── BPMN diagram
│       └── Operation history
│
├── Incidents
│   ├── All open incidents
│   ├── Grouping by process and version
│   └── Incident card
│
├── Connections
│   ├── Add and edit
│   ├── Connection check
│   ├── Automatic Cockpit link
│   └── Authorization
│
└── Settings
    ├── Appearance
    ├── Language
    ├── Local audit
    ├── Data masking
    └── App diagnostics
```

## 4. Main user scenarios

### Investigate a stuck instance

1. Pick a connection in the sidebar.
2. Find the instance by business key, ID or variable value.
3. If several processes match, pick the right instance.
4. Review the activity tree, the incident and the full error details.
5. On the same screen, check variables and the execution history.
6. If needed, use a separate button to open the same instance in Camunda Cockpit. Regular navigation stays inside the app.

### Retry a failed operation

1. Open an incident.
2. The app detects whether the failure belongs to an external task or an engine job.
3. It shows the error, retries and lock state.
4. It builds a preview of the operation.
5. For production, it requires a reason or ticket number.
6. It re-reads the entity state and runs the action only if the state has not changed.

### Teleport a token

1. Open a process instance and choose **Teleport**.
2. Pick a specific active `activityInstanceId`, not only a BPMN activity ID.
3. Pick the target activity from the deployed BPMN.
4. Choose `startBeforeActivity`, `startAfterActivity` or a specific sequence flow.
5. Set listener and I/O mapping behavior explicitly.
6. Review the source, the target, the affected scope and the REST payload.
7. Re-fetch the activity tree and make sure the state is not stale.
8. Run the modification and show the new process state right away.

### Share a link

Every instance has a `camunda-support://` deep link. Opening it in the app switches to the right connection and shows the instance. A Markdown report of the instance can be copied for tickets and chats.

## 5. Design concept

### Character

A calm, dense, operational interface. The app should feel like a professional control-room console, not a typical admin panel.

### Layout

- a single top bar: connection switcher, sections (Overview, Search, Incidents), the command palette (Cmd+K) and settings;
- the main canvas is designed for a wide desktop window;
- the instance view is split into tabs: Diagram, Timeline, Diagnostics, Variables, Metadata;
- the BPMN diagram takes the main area, the current state sits in a compact right column;
- clicking an element on the diagram starts a teleport to it;
- dangerous actions get their own clear confirmation flow;
- teleport shows two outlined lists, "From" and "To", both searchable.

### Language

The UI is English by default. Russian is available in Settings. All user-facing strings go through the locale layer, so adding a language does not touch feature code.

### Visual language

The look is built from the standard themes and components of GPUI Kit (see [design-system.md](design-system.md)). Colors carry meaning:

- green: healthy state and completed steps;
- amber: warnings and retryable failures;
- red: production, incidents and dangerous confirmation;
- technical IDs and numbers use a monospace font.

### Environment identification

The environment is always marked by both color and text:

| Environment | Color | Label |
|---|---:|---|
| Production | Red | `PRODUCTION` |
| Test | Amber | `TEST` |
| Development | Blue | `DEV` |
| Local | Neutral | `LOCAL` |

### Interaction principles

- the business key, the process instance ID and the environment are visible during any operation;
- dangerous actions are never hidden behind unclear icon-only buttons;
- an empty state explains the next useful step;
- IDs and error details have a copy action;
- a production operation requires review, a reason and explicit confirmation;
- a teleport target is picked by its business name, with the technical ID shown next to it.

## 6. Main entities

```text
CamundaConnection
├── name and environment
├── REST URL and an automatically derived Cockpit URL
├── credentials reference
└── instance lookup strategy

ProcessInstance
├── processInstanceId and businessKey
├── definition ID / key / version / versionTag
├── state and activity tree
├── variables and incidents
├── external tasks
└── jobs

SupportOperation
├── connection and process instance
├── operation type
├── state before and after
├── request and response
├── operator and reason
└── time and result
```

Example connection:

```text
name:      Orders (production)
REST URL:  https://camunda.example.com/engine-rest
Cockpit:   https://camunda.example.com/camunda/app/cockpit
```

## 7. Security boundaries

- the UI calls only typed commands, there is no generic HTTP executor;
- connection URLs are validated and the environment is always visible;
- secrets are not stored in connection records or audit logs;
- sensitive variables are masked by rule;
- preview and execution are separate commands;
- Camunda state is re-checked before execution;
- deleting deployments, arbitrary REST calls and bulk production changes are forbidden.
