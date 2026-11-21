function readWidth(key: string, fallback: number): number {
  try {
    const v = Number(localStorage.getItem(key));
    return Number.isFinite(v) && v > 0 ? v : fallback;
  } catch {
    return fallback;
  }
}

function writeWidth(key: string, value: number): void {
  try {
    localStorage.setItem(key, String(Math.round(value)));
  } catch {
    /* storage unavailable — width just won't persist */
  }
}

/**
 * Make `handle` resize `pane` horizontally. `side` is the edge of the pane the
 * handle sits on. Width is stored as a CSS variable on `host` and persisted.
 */
export function attachSplitter(opts: {
  handle: HTMLElement;
  pane: HTMLElement;
  host: HTMLElement;
  cssVar: string;
  storageKey: string;
  side: 'right' | 'left';
  min: number;
  max: number;
  fallback: number;
}): void {
  const { handle, pane, host, cssVar, storageKey, side, min, max, fallback } = opts;
  const clamp = (w: number) => Math.max(min, Math.min(max, w));
  host.style.setProperty(cssVar, `${clamp(readWidth(storageKey, fallback))}px`);

  handle.setAttribute('role', 'separator');
  handle.setAttribute('aria-orientation', 'vertical');
  handle.tabIndex = 0;

  handle.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = pane.getBoundingClientRect().width;
    let raf = 0;
    let latest = startW;
    handle.setPointerCapture(e.pointerId);
    handle.classList.add('dragging');
    document.body.classList.add('resizing');

    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - startX;
      latest = clamp(side === 'right' ? startW + dx : startW - dx);
      if (!raf) {
        raf = requestAnimationFrame(() => {
          raf = 0;
          host.style.setProperty(cssVar, `${latest}px`);
        });
      }
    };
    const up = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
      handle.removeEventListener('pointercancel', up);
      handle.classList.remove('dragging');
      document.body.classList.remove('resizing');
      writeWidth(storageKey, latest);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
    handle.addEventListener('pointercancel', up);
  });

  handle.addEventListener('keydown', (e) => {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    const cur = pane.getBoundingClientRect().width;
    const grow = (e.key === 'ArrowRight') === (side === 'right');
    const next = clamp(cur + (grow ? 16 : -16));
    host.style.setProperty(cssVar, `${next}px`);
    writeWidth(storageKey, next);
  });

  handle.addEventListener('dblclick', () => {
    host.style.setProperty(cssVar, `${fallback}px`);
    writeWidth(storageKey, fallback);
  });
}
