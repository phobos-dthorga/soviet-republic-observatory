export type ContrastAuditFailure = {
  selector: string;
  role: string;
  foreground: string;
  background: string;
  measured: number;
  required: number;
  text: string | undefined;
};

/**
 * Keep this function self-contained. Both browser and native review serialize
 * it into the inspected document so one computed-style rule set is exercised.
 */
export function auditInterfaceContrast(): ContrastAuditFailure[] {
  type Rgba = [number, number, number, number];
  const parse = (value: string): Rgba | null => {
    const match = value.match(/rgba?\(([^)]+)\)/);
    if (!match) return null;
    const channels = match[1].split(/[, ]+/).filter(Boolean).map(Number);
    return [channels[0], channels[1], channels[2], channels[3] ?? 1];
  };
  const composite = (top: Rgba, bottom: Rgba): Rgba => {
    const alpha = top[3] + bottom[3] * (1 - top[3]);
    if (alpha === 0) return [0, 0, 0, 0];
    return [
      (top[0] * top[3] + bottom[0] * bottom[3] * (1 - top[3])) / alpha,
      (top[1] * top[3] + bottom[1] * bottom[3] * (1 - top[3])) / alpha,
      (top[2] * top[3] + bottom[2] * bottom[3] * (1 - top[3])) / alpha,
      alpha,
    ];
  };
  const background = (element: Element): Rgba => {
    let result: Rgba = [255, 255, 255, 1];
    const layers: Rgba[] = [];
    let current: Element | null = element;
    while (current) {
      const parsed = parse(getComputedStyle(current).backgroundColor);
      if (parsed && parsed[3] > 0) layers.push(parsed);
      current = current.parentElement;
    }
    for (const layer of layers.reverse()) result = composite(layer, result);
    return result;
  };
  const linear = (channel: number) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  const luminance = (colour: Rgba) =>
    0.2126 * linear(colour[0]) +
    0.7152 * linear(colour[1]) +
    0.0722 * linear(colour[2]);
  const ratio = (first: Rgba, second: Rgba) => {
    const a = luminance(first);
    const b = luminance(second);
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  };
  const selector = [
    "body *:not(script):not(style):not(svg):not(path):not(canvas)",
    "option",
    "optgroup",
  ].join(",");
  return [...document.querySelectorAll(selector)].flatMap((element, index) => {
    const node = element as HTMLElement;
    const style = getComputedStyle(node);
    const visible =
      style.display !== "none" &&
      style.visibility !== "hidden" &&
      Number(style.opacity) > 0 &&
      (node.innerText?.trim() || node instanceof HTMLOptionElement);
    if (
      !visible ||
      [...element.children].some((child) =>
        (child as HTMLElement).innerText?.trim(),
      )
    ) {
      return [];
    }
    const foreground = parse(style.color);
    if (!foreground) return [];
    const effectiveBackground = background(element);
    const effectiveForeground = composite(foreground, effectiveBackground);
    const measured = ratio(effectiveForeground, effectiveBackground);
    const large =
      parseFloat(style.fontSize) >= 24 ||
      (parseFloat(style.fontSize) >= 18.66 && Number(style.fontWeight) >= 700);
    const required = large ? 3 : 4.5;
    if (measured + 0.01 >= required) return [];
    const identity = node.id
      ? `#${node.id}`
      : `${node.tagName.toLowerCase()}.${[...node.classList].join(".") || "unclassified"}:nth-audit(${index})`;
    return [
      {
        selector: identity,
        role: node.getAttribute("role") ?? node.tagName.toLowerCase(),
        foreground: style.color,
        background: style.backgroundColor,
        measured: Number(measured.toFixed(2)),
        required,
        text: node.innerText?.trim().slice(0, 100),
      },
    ];
  });
}
