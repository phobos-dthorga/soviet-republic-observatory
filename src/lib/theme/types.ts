export type ThemeColours = {
  canvas: string;
  surface: string;
  surface_raised: string;
  surface_soft: string;
  text: string;
  text_muted: string;
  line: string;
  accent: string;
  observed: string;
  risk: string;
  success: string;
  comparison: string;
};

export type ThemeManifest = {
  schema_version: 1;
  id: string;
  version: string;
  name: string;
  author?: string;
  description?: string;
  colours: ThemeColours;
  chart_palette: string[];
};

export type ThemeContrastCheck = {
  id: string;
  foreground: string;
  background: string;
  measured: number;
  minimum: number;
  passes: boolean;
  severity: "error" | "warning";
  remediation:
    | "increase_foreground_surface_difference"
    | "strengthen_control_boundary"
    | "strengthen_decorative_divider"
    | "strengthen_chart_surface_difference"
    | "adjust_derived_soft_fill"
    | "increase_chart_series_distinction";
};

export type ThemeValidationReport = {
  valid: boolean;
  native_colour_scheme: "dark" | "light";
  checks: ThemeContrastCheck[];
  errors: number;
  warnings: number;
};

export type ThemeInspection = {
  structurally_valid: boolean;
  code?: string;
  detail?: string;
  manifest?: ThemeManifest;
  content_hash?: string;
  report?: ThemeValidationReport;
};

export type AvailableThemeRevision = {
  manifest: ThemeManifest;
  content_hash: string;
  source: "built_in" | "local_import";
  installed_at_ms?: number;
  updated_at_ms?: number;
  selected: boolean;
  report: ThemeValidationReport;
};

export type ThemeStatus = {
  selected_theme_id: string;
  selected_version: string;
  selected_content_hash: string;
  active_theme: ThemeManifest;
  active_report: ThemeValidationReport;
  themes: AvailableThemeRevision[];
  fallback_applied: boolean;
  storage_authority: "native_sqlite";
};
