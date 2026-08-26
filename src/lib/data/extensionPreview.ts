import { RECEIVER_CORE_METRICS } from "../extensions/analysisPack";

export const receiverPackPreview = {
  id: "org.republic-observatory.examples.receiver-adoption-laboratory",
  name: "Receiver Adoption Laboratory",
  author: "Republic Observatory contributors",
  version: "1.0.0",
  hostApi: "1",
  validation: "Schema and semantic proof valid",
  inputs: [...RECEIVER_CORE_METRICS],
  derivedMetrics: [
    "classified_population",
    "none_share",
    "radio_share",
    "television_share",
    "computer_share",
  ],
  chart: "Receiver class shares · 100% stacked area",
  deniedCapabilities: [
    "Executable code",
    "Network access",
    "Raw-save access",
    "Custom interface code",
    "ECharts configuration",
  ],
} as const;
