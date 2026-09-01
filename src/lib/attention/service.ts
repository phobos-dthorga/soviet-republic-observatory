import { writable } from "svelte/store";
import {
  dismissNativeAttentionCue,
  getNativeAttentionCueStatus,
  nativeAttentionHostAvailable,
  replayNativeAttentionCue,
} from "./desktopClient";
import type { AttentionCueStatus } from "./types";

const fallbackDismissed = new Set<string>();
export const attentionRevision = writable(0);

function identity(cueId: string, contentRevision: number): string {
  return `${cueId}@${contentRevision}`;
}

export async function getAttentionCueStatus(
  cueId: string,
  contentRevision: number,
): Promise<AttentionCueStatus> {
  if (nativeAttentionHostAvailable()) {
    return getNativeAttentionCueStatus(cueId, contentRevision);
  }
  return {
    cue_id: cueId,
    content_revision: contentRevision,
    dismissed: fallbackDismissed.has(identity(cueId, contentRevision)),
  };
}

export async function dismissAttentionCue(
  cueId: string,
  contentRevision: number,
): Promise<AttentionCueStatus> {
  const status = nativeAttentionHostAvailable()
    ? await dismissNativeAttentionCue(cueId, contentRevision)
    : {
        cue_id: cueId,
        content_revision: contentRevision,
        dismissed: true,
      };
  fallbackDismissed.add(identity(cueId, contentRevision));
  attentionRevision.update((revision) => revision + 1);
  return status;
}

export async function replayAttentionCue(
  cueId: string,
  contentRevision: number,
): Promise<AttentionCueStatus> {
  const status = nativeAttentionHostAvailable()
    ? await replayNativeAttentionCue(cueId, contentRevision)
    : {
        cue_id: cueId,
        content_revision: contentRevision,
        dismissed: false,
      };
  fallbackDismissed.delete(identity(cueId, contentRevision));
  attentionRevision.update((revision) => revision + 1);
  return status;
}

export function noteAllAttentionCuesReplayed(): void {
  fallbackDismissed.clear();
  attentionRevision.update((revision) => revision + 1);
}
