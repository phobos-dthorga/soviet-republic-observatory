const WORKSPACE_CANVAS_SELECTOR = ".workspace > .canvas";

/**
 * Keeps in-workspace section links inside their own scrolling canvas.
 *
 * Native fragment navigation asks Chromium to scroll every ancestor, including
 * the otherwise hidden root document. In the desktop WebView that can move the
 * global command and save bars above the viewport with no player-facing way to
 * recover them.
 */
export function containedSectionNavigation(node: HTMLAnchorElement) {
  function navigate(event: MouseEvent): void {
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey
    ) {
      return;
    }

    const href = node.getAttribute("href");
    if (!href?.startsWith("#") || href.length === 1) return;

    // Even a temporarily unavailable section must not fall back to Chromium's
    // root-document fragment scrolling.
    event.preventDefault();

    const target = document.getElementById(decodeURIComponent(href.slice(1)));
    if (node.closest(".workspace") !== target?.closest(".workspace")) return;
    focusContainedWorkspaceTarget(target);
  }

  node.addEventListener("click", navigate);
  return {
    destroy() {
      node.removeEventListener("click", navigate);
    },
  };
}

export function focusContainedWorkspaceTarget(
  target: HTMLElement | null,
): boolean {
  const canvas = target?.closest<HTMLElement>(WORKSPACE_CANVAS_SELECTOR);
  if (!target || !canvas) return false;
  if (!target.hasAttribute("tabindex")) target.tabIndex = -1;
  target.focus({ preventScroll: true });

  const targetBox = target.getBoundingClientRect();
  const canvasBox = canvas.getBoundingClientRect();
  const top = Math.max(0, canvas.scrollTop + targetBox.top - canvasBox.top - 8);
  const reducedMotion = window.matchMedia(
    "(prefers-reduced-motion: reduce)",
  ).matches;
  canvas.scrollTo({ top, behavior: reducedMotion ? "auto" : "smooth" });
  return true;
}
