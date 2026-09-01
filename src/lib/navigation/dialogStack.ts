export type DialogRoute =
  | "language"
  | "theme"
  | "settings"
  | "observation"
  | "diagnostics"
  | "legal"
  | "research"
  | "recovery";

export function pushDialogRoute(
  stack: DialogRoute[],
  route: DialogRoute,
): DialogRoute[] {
  const existing = stack.indexOf(route);
  if (existing >= 0) return stack.slice(0, existing + 1);
  return [...stack, route];
}

export function removeDialogRoute(
  stack: DialogRoute[],
  route: DialogRoute,
): DialogRoute[] {
  return stack.filter((candidate) => candidate !== route);
}

export function topDialogRoute(stack: DialogRoute[]): DialogRoute | null {
  return stack.at(-1) ?? null;
}

export function dialogLayer(stack: DialogRoute[], route: DialogRoute): number {
  return stack.indexOf(route);
}
