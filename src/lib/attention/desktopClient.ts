import { invoke, isTauri } from "@tauri-apps/api/core";
import type { AttentionCueStatus } from "./types";

export function nativeAttentionHostAvailable(): boolean {
  return isTauri();
}

export function getNativeAttentionCueStatus(
  cueId: string,
  contentRevision: number,
): Promise<AttentionCueStatus> {
  return invoke<AttentionCueStatus>("attention_cue_status", {
    cueId,
    contentRevision,
  });
}

export function dismissNativeAttentionCue(
  cueId: string,
  contentRevision: number,
): Promise<AttentionCueStatus> {
  return invoke<AttentionCueStatus>("dismiss_attention_cue", {
    cueId,
    contentRevision,
  });
}

export function replayNativeAttentionCue(
  cueId: string,
  contentRevision: number,
): Promise<AttentionCueStatus> {
  return invoke<AttentionCueStatus>("replay_attention_cue", {
    cueId,
    contentRevision,
  });
}
