import { writable } from "svelte/store";
import type { ThemeManifest, ThemeValidationReport } from "./types";

export const activeTheme = writable<ThemeManifest | null>(null);

const variables: Record<keyof ThemeManifest["colours"], string> = {
  canvas: "--colour-canvas",
  surface: "--colour-surface",
  surface_raised: "--colour-surface-raised",
  surface_soft: "--colour-surface-soft",
  text: "--colour-text",
  text_muted: "--colour-muted",
  line: "--colour-line",
  accent: "--colour-gold",
  observed: "--colour-observed",
  risk: "--colour-risk",
  success: "--colour-success",
  comparison: "--colour-violet",
};

export function applyTheme(
  theme: ThemeManifest,
  report: ThemeValidationReport,
): void {
  activeTheme.set(theme);
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.dataset.theme = `${theme.id}@${theme.version}`;
  root.style.colorScheme = report.native_colour_scheme;
  for (const [key, variable] of Object.entries(variables)) {
    root.style.setProperty(
      variable,
      theme.colours[key as keyof ThemeManifest["colours"]],
    );
  }
  root.style.setProperty("--colour-text-muted", theme.colours.text_muted);
  root.style.setProperty("--colour-cyan", theme.colours.observed);
  root.style.setProperty(
    "--colour-line-faint",
    withAlpha(theme.colours.line, 0.45),
  );
  root.style.setProperty(
    "--colour-observed-soft",
    withAlpha(theme.colours.observed, 0.11),
  );
  root.style.setProperty(
    "--colour-gold-soft",
    withAlpha(theme.colours.accent, 0.11),
  );
  root.style.setProperty(
    "--colour-risk-soft",
    withAlpha(theme.colours.risk, 0.11),
  );
  root.style.setProperty(
    "--colour-success-soft",
    withAlpha(theme.colours.success, 0.11),
  );
  root.style.setProperty(
    "--colour-overlay",
    withAlpha(theme.colours.canvas, 0.94),
  );
  root.style.setProperty("--chart-colour-1", theme.chart_palette[0]);
  root.style.setProperty("--chart-colour-2", theme.chart_palette[1]);
  root.style.setProperty("--chart-colour-3", theme.chart_palette[2]);
  root.style.setProperty(
    "--chart-colour-4",
    theme.chart_palette[3] ?? theme.colours.success,
  );
  root.style.setProperty(
    "--chart-colour-5",
    theme.chart_palette[4] ?? theme.colours.comparison,
  );
}

function withAlpha(hex: string, alpha: number): string {
  const red = Number.parseInt(hex.slice(1, 3), 16);
  const green = Number.parseInt(hex.slice(3, 5), 16);
  const blue = Number.parseInt(hex.slice(5, 7), 16);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}
