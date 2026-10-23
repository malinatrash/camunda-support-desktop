import { cubicOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";

export const EASE_OUT = cubicOut;

export function easeOutQuint(t: number): number {
  return 1 - Math.pow(1 - t, 5);
}

export function easeSpring(t: number): number {
  const c4 = (2 * Math.PI) / 3;
  return t === 0 ? 0 : t === 1 ? 1 : Math.pow(2, -10 * t) * Math.sin((t * 10 - 0.75) * c4) + 1;
}

const reduced = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

// Появление экрана: лёгкий подъём с проявлением, без смещения layout.
export function screenIn(node: Element, { delay = 0, duration = 260 } = {}): TransitionConfig {
  if (reduced) return { duration: 0 };
  return {
    delay,
    duration,
    easing: easeOutQuint,
    css: (t) => `opacity: ${t}; transform: translateY(${(1 - t) * 10}px) scale(${0.992 + 0.008 * t});`,
  };
}

export function screenOut(node: Element, { duration = 140 } = {}): TransitionConfig {
  if (reduced) return { duration: 0 };
  return {
    duration,
    easing: cubicOut,
    css: (t) => `opacity: ${t}; transform: scale(${0.996 + 0.004 * t});`,
  };
}

// Появление строки списка или карточки: короткий подъём на 6px.
export function rise(node: Element, { delay = 0, duration = 220, y = 6 } = {}): TransitionConfig {
  if (reduced) return { duration: 0 };
  return {
    delay,
    duration,
    easing: easeOutQuint,
    css: (t) => `opacity: ${t}; transform: translateY(${(1 - t) * y}px);`,
  };
}

export function pop(node: Element, { delay = 0, duration = 200, start = 0.96 } = {}): TransitionConfig {
  if (reduced) return { duration: 0 };
  return {
    delay,
    duration,
    easing: easeOutQuint,
    css: (t) => `opacity: ${t}; transform: scale(${start + (1 - start) * t});`,
  };
}

export function fadeFast(node: Element, { duration = 140 } = {}): TransitionConfig {
  if (reduced) return { duration: 0 };
  return { duration, easing: cubicOut, css: (t) => `opacity: ${t};` };
}

// Плавное раскрытие по высоте для баннеров и панелей.
export function expand(node: Element, { duration = 240 } = {}): TransitionConfig {
  if (reduced) return { duration: 0 };
  const element = node as HTMLElement;
  const height = element.offsetHeight;
  return {
    duration,
    easing: easeOutQuint,
    css: (t) => `overflow: hidden; height: ${t * height}px; opacity: ${Math.min(1, t * 1.4)};`,
  };
}

export const flipOptions = { duration: 260, easing: easeOutQuint };
