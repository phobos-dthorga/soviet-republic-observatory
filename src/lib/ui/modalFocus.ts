export type ModalFocusOptions = {
  onclose: () => void;
  closeDisabled?: boolean;
  active?: boolean;
};

const focusableSelector =
  'button:not(:disabled), input:not(:disabled), select:not(:disabled), a[href], [tabindex]:not([tabindex="-1"])';

export function modalFocus(
  node: HTMLDialogElement,
  initialOptions: ModalFocusOptions,
) {
  let options = initialOptions;
  const previouslyFocused = document.activeElement as HTMLElement | null;
  let escapedNativeControl: HTMLElement | null = null;

  function focusableElements(): HTMLElement[] {
    return [...node.querySelectorAll<HTMLElement>(focusableSelector)].filter(
      (element) => !element.hasAttribute("hidden"),
    );
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (options.active === false) return;
    if (event.key === "Escape" && !options.closeDisabled) {
      const target = event.target;
      if (
        target instanceof HTMLSelectElement &&
        escapedNativeControl !== target
      ) {
        escapedNativeControl = target;
        return;
      }
      event.preventDefault();
      escapedNativeControl = null;
      options.onclose();
      return;
    }
    if (event.key !== "Escape") escapedNativeControl = null;
    if (event.key !== "Tab") return;
    const focusable = focusableElements();
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener("keydown", handleKeydown);
  function focusInitialControl(): void {
    if (options.active === false) return;
    const preferred = node.querySelector<HTMLElement>("[data-modal-autofocus]");
    const focusable = focusableElements();
    const target =
      preferred && focusable.includes(preferred) ? preferred : focusable[0];
    target?.focus();
  }

  queueMicrotask(focusInitialControl);

  return {
    update(nextOptions: ModalFocusOptions) {
      const becameActive =
        options.active === false && nextOptions.active !== false;
      options = nextOptions;
      if (becameActive) queueMicrotask(focusInitialControl);
    },
    destroy() {
      node.removeEventListener("keydown", handleKeydown);
      previouslyFocused?.focus();
    },
  };
}
