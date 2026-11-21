import { icon } from './icons';

const HTML_ESCAPES: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
};

/** Escape text for safe interpolation into HTML (content or quoted attributes). */
export function esc(text: unknown): string {
  return String(text ?? '').replace(/[&<>"']/g, (c) => HTML_ESCAPES[c]);
}

export function basename(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() || path;
}

export function dirname(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  parts.pop();
  return parts.join('/');
}

export type FileKind =
  | 'rust' | 'ts' | 'js' | 'json' | 'md' | 'css' | 'html' | 'py' | 'toml' | 'yaml' | 'shell' | 'text';

const EXT_KIND: Record<string, FileKind> = {
  rs: 'rust', ts: 'ts', tsx: 'ts', mts: 'ts', js: 'js', jsx: 'js', mjs: 'js', cjs: 'js',
  json: 'json', md: 'md', markdown: 'md', css: 'css', scss: 'css', html: 'html', htm: 'html',
  py: 'py', toml: 'toml', lock: 'toml', yaml: 'yaml', yml: 'yaml', sh: 'shell', bash: 'shell', ps1: 'shell',
};

export function fileKind(path: string): FileKind {
  const name = basename(path).toLowerCase();
  const ext = name.includes('.') ? name.split('.').pop()! : '';
  return EXT_KIND[ext] ?? 'text';
}

const KIND_LABEL: Record<FileKind, string> = {
  rust: 'Rust', ts: 'TypeScript', js: 'JavaScript', json: 'JSON', md: 'Markdown', css: 'CSS',
  html: 'HTML', py: 'Python', toml: 'TOML', yaml: 'YAML', shell: 'Shell', text: 'Plain Text',
};

export function kindLabel(kind: FileKind): string {
  return KIND_LABEL[kind];
}

export function fileSwatch(path: string): string {
  return `<span class="ftype" data-kind="${fileKind(path)}" aria-hidden="true"></span>`;
}

export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number): (...args: A) => void {
  let t: number | undefined;
  return (...args: A) => {
    window.clearTimeout(t);
    t = window.setTimeout(() => fn(...args), ms);
  };
}

// ── Toasts ──────────────────────────────────────────────────────────────────

let toastHost: HTMLElement | null = null;

export function toast(message: string, kind: 'ok' | 'err' | 'info' = 'ok', ms = 2400): void {
  if (!toastHost) {
    toastHost = document.createElement('div');
    toastHost.className = 'toasts';
    toastHost.setAttribute('role', 'status');
    toastHost.setAttribute('aria-live', 'polite');
    document.body.appendChild(toastHost);
  }
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.innerHTML = `${icon(kind === 'ok' ? 'check' : kind === 'err' ? 'alert' : 'info')}<span>${esc(message)}</span>`;
  toastHost.appendChild(el);
  window.setTimeout(() => {
    el.classList.add('leaving');
    window.setTimeout(() => el.remove(), 220);
  }, ms);
}

// ── Dialogs (replace window.prompt / confirm, which look out of place) ─────

interface DialogOptions {
  title: string;
  message?: string;
  value?: string;
  placeholder?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  input?: boolean;
  danger?: boolean;
}

function openDialog(opts: DialogOptions): Promise<string | null> {
  return new Promise((resolve) => {
    const previousFocus = document.activeElement as HTMLElement | null;
    const overlay = document.createElement('div');
    overlay.className = 'dialog-overlay';
    overlay.innerHTML = `
      <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title">
        <h2 id="dlg-title">${esc(opts.title)}</h2>
        ${opts.message ? `<p>${esc(opts.message)}</p>` : ''}
        ${opts.input ? `<label class="sr-only" for="dlg-input">${esc(opts.title)}</label><input id="dlg-input" spellcheck="false" autocomplete="off" placeholder="${esc(opts.placeholder ?? '')}" value="${esc(opts.value ?? '')}">` : ''}
        <div class="dialog-actions">
          <button type="button" class="btn" data-act="cancel">${esc(opts.cancelLabel ?? 'Cancel')}</button>
          <button type="button" class="btn btn-primary" data-act="ok">${esc(opts.confirmLabel ?? 'OK')}</button>
        </div>
      </div>`;
    document.body.appendChild(overlay);

    const input = overlay.querySelector<HTMLInputElement>('input');
    const okBtn = overlay.querySelector<HTMLButtonElement>('[data-act="ok"]')!;

    const finish = (result: string | null) => {
      overlay.remove();
      previousFocus?.focus?.();
      resolve(result);
    };

    overlay.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      if (target === overlay) finish(null);
      const act = target.closest<HTMLElement>('[data-act]')?.dataset.act;
      if (act === 'cancel') finish(null);
      if (act === 'ok') finish(input ? input.value.trim() || null : 'ok');
    });
    overlay.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        finish(null);
      } else if (e.key === 'Enter' && (input ? document.activeElement === input : true)) {
        e.preventDefault();
        e.stopPropagation();
        finish(input ? input.value.trim() || null : 'ok');
      }
    });

    window.setTimeout(() => {
      if (input) {
        input.focus();
        input.select();
      } else {
        okBtn.focus();
      }
    }, 0);
  });
}

export function askText(title: string, opts: Omit<DialogOptions, 'title' | 'input'> = {}): Promise<string | null> {
  return openDialog({ ...opts, title, input: true });
}

export async function confirmDialog(title: string, opts: Omit<DialogOptions, 'title' | 'input'> = {}): Promise<boolean> {
  return (await openDialog({ ...opts, title, input: false })) !== null;
}

// ── Lightweight markdown for agent output ──────────────────────────────────

function inlineMd(text: string): string {
  let out = esc(text);
  out = out.replace(/`([^`\n]+)`/g, '<code class="inline-code">$1</code>');
  out = out.replace(/\*\*([^*\n]+)\*\*/g, '<strong>$1</strong>');
  return out;
}

/** Render a safe subset of markdown: fenced code, inline code, bold, lists, paragraphs. */
export function renderMarkdown(src: string): string {
  if (!src) return '';
  const parts = src.split(/(```[\s\S]*?(?:```|$))/g);
  return parts
    .map((part) => {
      if (part.startsWith('```')) {
        const body = part.replace(/^```/, '').replace(/```$/, '');
        const nl = body.indexOf('\n');
        const lang = nl === -1 ? '' : body.slice(0, nl).trim();
        const code = nl === -1 ? body : body.slice(nl + 1);
        return `<div class="code-block"><div class="cb-head"><span>${esc(lang || 'code')}</span><button type="button" class="icon-btn" data-copy aria-label="Copy code">${icon('copy')}</button></div><pre><code>${esc(code.replace(/\n$/, ''))}</code></pre></div>`;
      }
      return part
        .split(/\n{2,}/)
        .map((block) => block.trim())
        .filter(Boolean)
        .map((block) => {
          const lines = block.split('\n');
          if (lines.every((l) => /^\s*([-*]|\d+\.)\s+/.test(l))) {
            const ordered = /^\s*\d+\./.test(lines[0]);
            const items = lines.map((l) => `<li>${inlineMd(l.replace(/^\s*([-*]|\d+\.)\s+/, ''))}</li>`).join('');
            return ordered ? `<ol>${items}</ol>` : `<ul>${items}</ul>`;
          }
          return `<p>${lines.map(inlineMd).join('<br>')}</p>`;
        })
        .join('');
    })
    .join('');
}

/** Delegate clicks on `[data-copy]` buttons inside rendered markdown. */
export function wireCopyButtons(root: HTMLElement): void {
  root.addEventListener('click', (e) => {
    const btn = (e.target as HTMLElement).closest<HTMLElement>('[data-copy]');
    if (!btn) return;
    const code = btn.closest('.code-block')?.querySelector('code')?.textContent ?? '';
    void navigator.clipboard?.writeText(code).then(
      () => toast('Copied to clipboard', 'ok', 1400),
      () => toast('Clipboard unavailable', 'err')
    );
  });
}

/** Summarise a tool call's arguments into one short line. */
export function summarizeArgs(args: unknown): string {
  if (args == null) return '';
  if (typeof args === 'string') return args;
  if (typeof args === 'object') {
    const obj = args as Record<string, unknown>;
    const preferred = ['path', 'file', 'query', 'pattern', 'command', 'cmd', 'dir'];
    for (const key of preferred) {
      if (typeof obj[key] === 'string') return String(obj[key]);
    }
    const first = Object.values(obj).find((v) => typeof v === 'string');
    if (typeof first === 'string') return first;
    try {
      return JSON.stringify(obj);
    } catch {
      return '';
    }
  }
  return String(args);
}
