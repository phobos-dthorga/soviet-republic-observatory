export type TaskRunState = "running" | "complete" | "failed";
export type TaskStageState = "pending" | "active" | "complete" | "failed";

export type TimestampedTaskProgress = {
  started_at_ms: number | null;
  updated_at_ms: number | null;
};

export type TaskProgressStage = {
  id: string;
  label: string;
  state: TaskStageState;
  stateLabel: string;
};

export type TaskProgressMetric = {
  id: string;
  label: string;
  value: string;
};

export type TaskProgressMeter = {
  id: string;
  label: string;
  completed: number;
  total: number;
};

export type TaskProgressNotice = {
  tone: "warning" | "error";
  text: string;
};

export type TaskProgressView = {
  taskId: string;
  runId: string;
  state: TaskRunState;
  eyebrow: string;
  heading: string;
  progressPercent: number | null;
  stages: TaskProgressStage[];
  metrics: TaskProgressMetric[];
  meters: TaskProgressMeter[];
  currentItemLabel: string | null;
  currentItem: string | null;
  currentItemContext: string | null;
  notice: TaskProgressNotice | null;
};

export type TaskProgressSource<T extends TimestampedTaskProgress> = {
  read: () => Promise<T>;
  listen: (accept: (progress: T) => void) => Promise<() => void>;
};

export function selectLatestTaskProgress<T extends TimestampedTaskProgress>(
  current: T | null,
  candidate: T,
): T {
  if (!current) return candidate;

  const currentStarted = current.started_at_ms ?? -1;
  const candidateStarted = candidate.started_at_ms ?? -1;
  if (candidateStarted !== currentStarted) {
    return candidateStarted > currentStarted ? candidate : current;
  }

  const currentUpdated = current.updated_at_ms ?? -1;
  const candidateUpdated = candidate.updated_at_ms ?? -1;
  return candidateUpdated >= currentUpdated ? candidate : current;
}

/**
 * Registers the event listener before reading the durable snapshot, then rejects
 * any older snapshot that arrives after a live update. All critical native tasks
 * should use this hand-off so work started during application boot stays visible.
 */
export async function observeLatestTaskProgress<
  T extends TimestampedTaskProgress,
>(
  source: TaskProgressSource<T>,
  accept: (progress: T) => void,
  onReadError?: (error: unknown) => void,
): Promise<() => void> {
  let latest: T | null = null;
  let stopped = false;
  const publish = (candidate: T): void => {
    if (stopped) return;
    const selected = selectLatestTaskProgress(latest, candidate);
    if (selected === latest) return;
    latest = selected;
    accept(selected);
  };

  const unlisten = await source.listen(publish);
  void source
    .read()
    .then(publish)
    .catch(onReadError ?? (() => undefined));

  return () => {
    stopped = true;
    unlisten();
  };
}
