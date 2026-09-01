type NavigationGuard = () => boolean;

const guards = new Map<string, NavigationGuard>();

export function registerNavigationGuard(
  id: string,
  guard: NavigationGuard,
): () => void {
  guards.set(id, guard);
  return () => {
    if (guards.get(id) === guard) guards.delete(id);
  };
}

export function hasUnsavedNavigationChanges(): boolean {
  return [...guards.values()].some((guard) => guard());
}
