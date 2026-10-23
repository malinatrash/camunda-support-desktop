// Переносит узел в body, чтобы модальные окна и всплывашки не зависели от stacking context родителя.
export function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
}

export function clickOutside(node: HTMLElement, onOutside: () => void) {
  const handler = (event: PointerEvent) => {
    if (!node.contains(event.target as Node)) onOutside();
  };
  document.addEventListener("pointerdown", handler, true);
  return {
    update(next: () => void) {
      onOutside = next;
    },
    destroy() {
      document.removeEventListener("pointerdown", handler, true);
    },
  };
}

// Фокусирует поле после появления, если enabled; без a11y-атрибута autofocus.
export function focusOnMount(node: HTMLElement, enabled: boolean) {
  if (enabled) requestAnimationFrame(() => node.focus());
}
