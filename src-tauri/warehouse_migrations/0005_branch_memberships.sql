CREATE TABLE branch_observation_memberships (
    branch_id VARCHAR NOT NULL,
    membership_revision BIGINT NOT NULL,
    interpretation_id VARCHAR NOT NULL,
    payload_hash VARCHAR NOT NULL,
    parent_interpretation_id VARCHAR,
    relationship VARCHAR NOT NULL,
    shared_record_count BIGINT NOT NULL,
    PRIMARY KEY (branch_id, membership_revision, interpretation_id)
);

CREATE TABLE branch_membership_generations (
    branch_id VARCHAR NOT NULL,
    membership_revision BIGINT NOT NULL,
    projected_at_ms BIGINT NOT NULL,
    PRIMARY KEY (branch_id, membership_revision)
);

CREATE VIEW current_branch_observation_memberships AS
SELECT membership.*
FROM branch_observation_memberships membership
JOIN (
    SELECT branch_id, MAX(membership_revision) AS membership_revision
    FROM branch_membership_generations
    GROUP BY branch_id
) latest USING (branch_id, membership_revision);
