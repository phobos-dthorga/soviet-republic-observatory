import type { ThemeManifest } from "../theme/types";
import { observatoryChartTheme } from "./chartOptions";
import type { ChartTheme } from "./types";

/** Maps a validated host theme into the bounded application-owned chart model. */
export function chartThemeFor(theme: ThemeManifest | null): ChartTheme {
  if (!theme) return observatoryChartTheme;
  return {
    palette: [...theme.chart_palette],
    text: theme.colours.text,
    muted: theme.colours.text_muted,
    line: theme.colours.line,
    tooltipBackground: theme.colours.surface_raised,
    tooltipBorder: theme.colours.line,
  };
}
