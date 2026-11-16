# Roadmap

Status as of 5 November 2026, version 2.3.

## Done

- Native Rust interface (GPUI + GPUI Kit) for macOS, Windows and Ubuntu. The `camunda-core` crate has no UI dependencies.
- English UI by default, Russian available in Settings.
- Connections: `/engine-rest` and `/version` check, Cockpit URL derived from the REST domain, import of 1.x settings.
- Overview: incidents, instances in progress and stuck ones, error groups, long waits. Processes by key with versions ordered by deploy date. Periods: Now, Today, Yesterday.
- Process view: all versions or a single one, virtualized instance table.
- Search by business key, ID and variables, with a variable key catalog cached for 24 hours.
- Incidents: groups and list, filters, bulk retry with confirmation for production.
- Instance: native BPMN diagram with token state highlighting and zoom, timeline, diagnostics (job and external task retries, unlock), variables with editing and type validation, suspend and activate, Markdown report, `camunda-support://` deep link.
- Teleport: click an element on the diagram, pick the exact activity instance, find the target by name, ID, type and topic, reason and separate confirmation for production.
- Command palette (Cmd+K) and section hotkeys.
- Auto-update from GitHub Releases with SHA-256 verification. DMG, NSIS and deb packages are built in CI on a tag.

## Next

### Investigation

- Drilldown into embedded and called subprocesses on the diagram.
- Full job stacktraces and external task error details.
- Local variables per activity and execution scope.
- Linked parent and child process instances.
- Pinned and recent instances, instance tabs across connections.

### Security and distribution

- Basic and Bearer authorization for connections, secrets in Keychain and Credential Manager.
- Masking of sensitive variables (personal IDs, OTP codes, tokens, documents).
- Signed packages: Developer ID and notarization on macOS, a certificate on Windows.
- Single app instance on Windows and Linux, so a deep link opens in the running window.

### Recovery

- A mandatory reason for every mutating operation and a local audit log.
- Configurable retry count instead of a fixed three.
- `startAfterActivity`, `startTransition` and variables in teleport instructions.
- Activity tree comparison before and after an operation.

## Later

- Restart of completed and canceled instances.
- Migration between process definitions.
- Recovery playbooks approved by the organization.
- Analytics of frequent incidents and read-only reports.
