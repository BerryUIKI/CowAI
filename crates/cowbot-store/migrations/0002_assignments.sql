-- Assignment runtime: task execution tree and append-only transition events.

CREATE TABLE assignments (
    id TEXT PRIMARY KEY NOT NULL,
    root_id TEXT NOT NULL,
    parent_id TEXT,
    depth INTEGER NOT NULL,
    task_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    requester TEXT NOT NULL,
    assignee TEXT NOT NULL,
    mode TEXT NOT NULL,
    reply TEXT NOT NULL,
    status TEXT NOT NULL,
    executions INTEGER NOT NULL,
    restarts INTEGER NOT NULL,
    wakes INTEGER NOT NULL,
    request_budget INTEGER NOT NULL,
    execution_budget INTEGER NOT NULL,
    wake_budget INTEGER NOT NULL,
    approval_level TEXT NOT NULL,
    outcome TEXT,
    stall_strikes INTEGER NOT NULL DEFAULT 0,
    repeated_errors INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES assignments(id) ON DELETE CASCADE
);

CREATE INDEX idx_assignments_task ON assignments(task_id);
CREATE INDEX idx_assignments_root ON assignments(root_id);
CREATE INDEX idx_assignments_parent ON assignments(parent_id);
CREATE INDEX idx_assignments_status ON assignments(status);

CREATE TABLE assignment_events (
    id TEXT PRIMARY KEY NOT NULL,
    assignment_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    detail TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (assignment_id) REFERENCES assignments(id) ON DELETE CASCADE
);

CREATE INDEX idx_assignment_events_assignment
    ON assignment_events(assignment_id, created_at);
