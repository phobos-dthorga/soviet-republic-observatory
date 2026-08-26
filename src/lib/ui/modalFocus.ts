export type ModalFocusOptions = {
  onclose: () => void;
  closeDisabled?: boolean;
};

const focusableSelector =
  'button:not(:disabled), input:not(:disabled), select:not(:disabled), a[href], [tabindex]:not([tabindex="-1"])';

export function modalFocus(
  node: HTMLDialogElement,
  initialOptions: ModalFocusOptions,
) {
  let options = initialOptions;
  const previouslyFocused = document.activeElement as HTMLElement | null;

  function focusableElements(): HTMLElement[] {
    return [...node.querySelectorAll<HTMLElement>(focusableSelector)].filter(
      (element) => !element.hasAttribute("hidden"),
    );
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" && !options.closeDisabled) {
      event.preventDefault();
      options.onclose();
      return;
    }
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
  queueMicrotask(() => {
    const preferred = node.querySelector<HTMLElement>("[data-modal-autofocus]");
    (preferred ?? focusableElements()[0])?.focus();
  });

  return {
    update(nextOptions: ModalFocusOptions) {
      options = nextOptions;
    },
    destroy() {
      node.removeEventListener("keydown", handleKeydown);
      previouslyFocused?.focus();
    },
  };
}
