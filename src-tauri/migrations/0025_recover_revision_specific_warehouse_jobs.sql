UPDATE warehouse_projection_jobs
SET status = 'pending', started_at_ms = NULL, applied_at_ms = NULL, error_code = NULL
WHERE projection_kind = 'branch_membership'
   OR (projection_kind = 'environment_observation' AND status = 'failed');
