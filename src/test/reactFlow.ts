// jsdom shims React Flow needs to mount: ResizeObserver, DOMMatrixReadOnly, and a 1,000 × 700 bounding box so the
// viewport and node measurement see a real size. Import it first in a test file that renders React Flow.
class ResizeObserverShim {
  private readonly cb: ResizeObserverCallback;
  constructor(cb: ResizeObserverCallback) {
    this.cb = cb;
  }
  observe(target: Element): void {
    this.cb([{ target, contentRect: target.getBoundingClientRect() } as unknown as ResizeObserverEntry], this);
  }
  unobserve(): void {}
  disconnect(): void {}
}

class DOMMatrixReadOnlyShim {
  m22: number;
  constructor(transform?: string) {
    const scale = transform?.match(/scale\(([1-9.])\)/)?.[1];
    this.m22 = scale !== undefined ? +scale : 1;
  }
}

if (typeof window !== "undefined") {
  window.ResizeObserver = ResizeObserverShim as unknown as typeof ResizeObserver;
  (window as unknown as { DOMMatrixReadOnly: unknown }).DOMMatrixReadOnly = DOMMatrixReadOnlyShim;
  HTMLElement.prototype.getBoundingClientRect = function () {
    return { x: 0, y: 0, top: 0, left: 0, bottom: 700, right: 1000, width: 1000, height: 700, toJSON: () => ({}) };
  };
  Object.defineProperties(HTMLElement.prototype, {
    offsetWidth: { configurable: true, get: () => 1000 },
    offsetHeight: { configurable: true, get: () => 700 },
  });
  (SVGElement.prototype as unknown as { getBBox: () => DOMRect }).getBBox = () =>
    ({ x: 0, y: 0, width: 0, height: 0 }) as DOMRect;
}

export {};
