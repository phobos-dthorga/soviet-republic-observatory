export type ContextHelpDetail = {
  label: string;
  value: string;
};

export type ContextHelpContent = {
  topic: string;
  title: string;
  text: string;
  details?: ContextHelpDetail[];
};
