import { get } from "svelte/store";
import { applyWordingMode } from "../i18n/runtime";
import type { WordingMode } from "../settings/types";
import {
  applyReviewTextScale,
  applyTheme,
  clearReviewTextScale,
} from "../theme/runtime";
import { selectTheme, themeStatus } from "../theme/service";
import { getUiReviewContext, type UiReviewContext } from "./desktopClient";
import {
  UI_REVIEW_SCENARIOS,
  type UiReviewScenarioId,
  type UiReviewThemeId,
} from "./scenarios";

export type { UiReviewScenarioId, UiReviewThemeId } from "./scenarios";

export type UiReviewScenarioRequest = {
  scenario: UiReviewScenarioId;
};

export type UiReviewController = {
  context: UiReviewContext;
  listScenarios(): readonly UiReviewScenarioId[];
  selectScenario(scenario: UiReviewScenarioId): Promise<void>;
  selectTheme(theme: UiReviewThemeId): Promise<void>;
  setTextScale(percent: number): Promise<void>;
  setWordingMode(mode: WordingMode): Promise<void>;
  settle(): Promise<void>;
};

declare global {
  interface Window {
    __REPUBLIC_OBSERVATORY_UI_REVIEW__?: UiReviewController;
  }
}

export async function initialiseUiReview(
  applyScenario: (request: UiReviewScenarioRequest) => void | Promise<void>,
): Promise<() => void> {
  const context = await getUiReviewContext();
  if (!context.enabled) return () => undefined;

  const controller: UiReviewController = {
    context,
    listScenarios: () => UI_REVIEW_SCENARIOS,
    selectScenario: async (scenario) => {
      if (!UI_REVIEW_SCENARIOS.includes(scenario)) {
        throw new Error("ui_review_unknown_scenario");
      }
      await applyScenario({ scenario });
      document.documentElement.dataset.uiReviewScenario = scenario;
      await settleInterface();
    },
    selectTheme: async (theme) => {
      if (theme === "boundary") {
        if (
          !context.validator_boundary_theme ||
          !context.validator_boundary_report
        ) {
          throw new Error("ui_review_boundary_theme_unavailable");
        }
        applyTheme(
          context.validator_boundary_theme,
          context.validator_boundary_report,
        );
      } else {
        const status = get(themeStatus);
        const expectedId =
          theme === "classic"
            ? "org.republic-observatory.classic"
            : "org.republic-observatory.high-contrast";
        const revision = status?.themes.find(
          (candidate) => candidate.manifest.id === expectedId,
        );
        if (!revision) throw new Error("ui_review_theme_unavailable");
        await selectTheme(
          revision.manifest.id,
          revision.manifest.version,
          revision.content_hash,
        );
      }
      await settleInterface();
    },
    setTextScale: async (percent) => {
      if (!Number.isInteger(percent) || percent < 100 || percent > 200) {
        throw new Error("ui_review_invalid_text_scale");
      }
      applyReviewTextScale(percent);
      document.documentElement.dataset.uiReviewTextScale = String(percent);
      await settleInterface();
    },
    setWordingMode: async (mode) => {
      if (mode !== "player_friendly" && mode !== "technical") {
        throw new Error("ui_review_invalid_wording_mode");
      }
      applyWordingMode(mode);
      await settleInterface();
    },
    settle: settleInterface,
  };

  window.__REPUBLIC_OBSERVATORY_UI_REVIEW__ = controller;
  document.documentElement.dataset.uiReview = context.data_state ?? "fixture";
  await settleInterface();
  return () => {
    delete window.__REPUBLIC_OBSERVATORY_UI_REVIEW__;
    delete document.documentElement.dataset.uiReview;
    delete document.documentElement.dataset.uiReviewScenario;
    delete document.documentElement.dataset.uiReviewTextScale;
    clearReviewTextScale();
  };
}

async function settleInterface(): Promise<void> {
  document.documentElement.dataset.uiReviewReady = "false";
  await document.fonts.ready;
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const deadline = performance.now() + 5_000;
  while (
    document.querySelector("[aria-busy='true']") &&
    performance.now() < deadline
  ) {
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  document.documentElement.dataset.uiReviewReady = "true";
}
