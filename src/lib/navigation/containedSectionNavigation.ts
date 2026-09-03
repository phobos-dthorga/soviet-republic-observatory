const WORKSPACE_CANVAS_SELECTOR = ".workspace > .canvas";

function ensureCurrentSection(workspace: HTMLElement): void {
  const links = [
    ...workspace.querySelectorAll<HTMLAnchorElement>(".section-list a"),
  ];
  const current = links.find(
    (link) => link.getAttribute("aria-current") === "location",
  );
  if (!current) links[0]?.setAttribute("aria-current", "location");
}

/**
 * Keeps in-workspace section links inside their own scrolling canvas.
 *
 * Native fragment navigation asks Chromium to scroll every ancestor, including
 * the otherwise hidden root document. In the desktop WebView that can move the
 * global command and save bars above the viewport with no player-facing way to
 * recover them.
 */
export function containedSectionNavigation(node: HTMLAnchorElement) {
  const workspace = node.closest<HTMLElement>(".workspace");
  const canvas = workspace?.querySelector<HTMLElement>(":scope > .canvas");
  const href = node.getAttribute("href");
  const target = href?.startsWith("#")
    ? document.getElementById(decodeURIComponent(href.slice(1)))
    : null;
  let releaseScrollLock: (() => void) | null = null;

  function lockCurrentSection(requestedHref: string): void {
    if (!workspace || !canvas) return;
    releaseScrollLock?.();
    workspace.dataset.sectionNavigationTarget = requestedHref;
    let timeout = 0;
    const release = () => {
      if (workspace.dataset.sectionNavigationTarget === requestedHref) {
        delete workspace.dataset.sectionNavigationTarget;
      }
      if (timeout) window.clearTimeout(timeout);
      if (releaseScrollLock === release) releaseScrollLock = null;
    };
    releaseScrollLock = release;
    timeout = window.setTimeout(release, 1_200);
  }

  function markCurrent(): void {
    if (!workspace) return;
    for (const link of workspace.querySelectorAll(
      ".section-list a[aria-current='location']",
    )) {
      link.removeAttribute("aria-current");
    }
    node.setAttribute("aria-current", "location");
  }

  const currentSectionObserver = workspace
    ? new MutationObserver(() => ensureCurrentSection(workspace))
    : null;
  if (currentSectionObserver && workspace) {
    currentSectionObserver.observe(workspace, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["aria-current"],
    });
  }
  if (workspace) queueMicrotask(() => ensureCurrentSection(workspace));

  const observer =
    canvas && target && target.closest(".workspace") === workspace
      ? new IntersectionObserver(
          (entries) => {
            const requestedHref = workspace?.dataset.sectionNavigationTarget;
            if (requestedHref && requestedHref !== href) return;
            if (entries.some((entry) => entry.isIntersecting)) markCurrent();
          },
          {
            root: canvas,
            rootMargin: "-4px 0px -65% 0px",
            threshold: 0.01,
          },
        )
      : null;
  if (observer && target) observer.observe(target);

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

    const requestedHref = node.getAttribute("href");
    if (!requestedHref?.startsWith("#") || requestedHref.length === 1) return;

    // Even a temporarily unavailable section must not fall back to Chromium's
    // root-document fragment scrolling.
    event.preventDefault();

    const requestedTarget = document.getElementById(
      decodeURIComponent(requestedHref.slice(1)),
    );
    if (node.closest(".workspace") !== requestedTarget?.closest(".workspace"))
      return;
    lockCurrentSection(requestedHref);
    markCurrent();
    focusContainedWorkspaceTarget(requestedTarget);
  }

  node.addEventListener("click", navigate);
  return {
    destroy() {
      observer?.disconnect();
      currentSectionObserver?.disconnect();
      releaseScrollLock?.();
      if (workspace) queueMicrotask(() => ensureCurrentSection(workspace));
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
