CREATE TABLE republic_plans (
    plan_id TEXT PRIMARY KEY
        CHECK (length(plan_id) BETWEEN 8 AND 64),
    display_name TEXT NOT NULL
        CHECK (length(display_name) BETWEEN 1 AND 120),
    branch_id TEXT NOT NULL
        REFERENCES timeline_branches(branch_id),
    active_revision INTEGER NOT NULL CHECK (active_revision > 0),
    removed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE (plan_id, branch_id)
) STRICT;

CREATE INDEX republic_plans_branch
    ON republic_plans(branch_id, removed_at_ms, updated_at_ms);

CREATE TABLE republic_plan_revisions (
    plan_id TEXT NOT NULL
        REFERENCES republic_plans(plan_id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision > 0),
    display_name TEXT NOT NULL
        CHECK (length(display_name) BETWEEN 1 AND 120),
    branch_id TEXT NOT NULL
        REFERENCES timeline_branches(branch_id),
    start_interpretation_id TEXT NOT NULL,
    start_profile_hash TEXT NOT NULL CHECK (length(start_profile_hash) = 64),
    start_year INTEGER NOT NULL,
    start_day INTEGER NOT NULL CHECK (start_day BETWEEN 0 AND 364),
    start_game_day INTEGER NOT NULL,
    end_year INTEGER NOT NULL,
    end_day INTEGER NOT NULL CHECK (end_day BETWEEN 0 AND 364),
    end_game_day INTEGER NOT NULL,
    schedule_kind TEXT NOT NULL CHECK (
        schedule_kind IN ('linear', 'milestone', 'hold_then_change')
    ),
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (plan_id, revision),
    UNIQUE (branch_id, plan_id, revision),
    FOREIGN KEY (plan_id, branch_id)
        REFERENCES republic_plans(plan_id, branch_id) ON DELETE CASCADE,
    CHECK (end_game_day > start_game_day)
) STRICT;

CREATE TABLE republic_plan_targets (
    plan_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal BETWEEN 0 AND 11),
    metric_id TEXT NOT NULL CHECK (length(metric_id) BETWEEN 3 AND 128),
    baseline_value INTEGER NOT NULL CHECK (baseline_value >= 0),
    target_value INTEGER NOT NULL CHECK (target_value >= 0),
    direction TEXT NOT NULL CHECK (
        direction IN ('increase', 'decrease', 'maintain')
    ),
    guardrail_basis_points INTEGER NOT NULL
        CHECK (guardrail_basis_points BETWEEN 0 AND 5000),
    PRIMARY KEY (plan_id, revision, ordinal),
    UNIQUE (plan_id, revision, metric_id),
    FOREIGN KEY (plan_id, revision)
        REFERENCES republic_plan_revisions(plan_id, revision)
        ON DELETE CASCADE
) STRICT;

CREATE TABLE active_republic_plans (
    branch_id TEXT PRIMARY KEY
        REFERENCES timeline_branches(branch_id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0),
    selected_at_ms INTEGER NOT NULL,
    FOREIGN KEY (branch_id, plan_id, revision)
        REFERENCES republic_plan_revisions(branch_id, plan_id, revision)
) STRICT;
