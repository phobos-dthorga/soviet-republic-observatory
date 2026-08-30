export type AttentionCueStatus = {
  cue_id: string;
  content_revision: number;
  dismissed: boolean;
};

export type AttentionCueTone = "information" | "important" | "success";
