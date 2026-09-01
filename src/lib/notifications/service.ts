import { writable } from "svelte/store";

export type NotificationTone = "info" | "success" | "warning" | "error";

export type RecoveryProposal = {
  title: string;
  message: string;
  consequence?: string;
  actionLabel: string;
  run: () => void | Promise<void>;
};

export type AppNotification = {
  id: string;
  title?: string;
  message: string;
  tone: NotificationTone;
  createdAt: number;
  timeoutMs: number;
  dedupeKey?: string;
  recovery?: RecoveryProposal;
};

export type NotificationRequest = {
  title?: string;
  message: string;
  tone?: NotificationTone;
  timeoutMs?: number;
  dedupeKey?: string;
  recovery?: RecoveryProposal;
};

const MAX_VISIBLE_NOTIFICATIONS = 5;
const DEFAULT_TIMEOUTS: Record<NotificationTone, number> = {
  info: 8_000,
  success: 6_000,
  warning: 10_000,
  error: 0,
};

let sequence = 0;
const notificationStore = writable<AppNotification[]>([]);
const recoveryProposalStore = writable<RecoveryProposal | null>(null);

export const notifications = {
  subscribe: notificationStore.subscribe,
};

export const recoveryProposal = {
  subscribe: recoveryProposalStore.subscribe,
};

export function notify(request: NotificationRequest): string {
  const tone = request.tone ?? "info";
  let notificationId = `notification-${++sequence}`;
  notificationStore.update((current) => {
    const existing = request.dedupeKey
      ? current.find(
          (notification) => notification.dedupeKey === request.dedupeKey,
        )
      : undefined;
    if (existing) notificationId = existing.id;
    const notification: AppNotification = {
      id: notificationId,
      title: request.title,
      message: request.message,
      tone,
      createdAt: Date.now(),
      timeoutMs: request.timeoutMs ?? DEFAULT_TIMEOUTS[tone],
      dedupeKey: request.dedupeKey,
      recovery: request.recovery,
    };
    const withoutDuplicate = existing
      ? current.filter((item) => item.id !== existing.id)
      : current;
    return [...withoutDuplicate, notification].slice(
      -MAX_VISIBLE_NOTIFICATIONS,
    );
  });
  return notificationId;
}

export function openRecoveryProposal(proposal: RecoveryProposal): void {
  recoveryProposalStore.set(proposal);
}

export function dismissRecoveryProposal(): void {
  recoveryProposalStore.set(null);
}

export function dismissNotification(id: string): void {
  notificationStore.update((current) =>
    current.filter((notification) => notification.id !== id),
  );
}

export function clearNotifications(): void {
  notificationStore.set([]);
  recoveryProposalStore.set(null);
}
