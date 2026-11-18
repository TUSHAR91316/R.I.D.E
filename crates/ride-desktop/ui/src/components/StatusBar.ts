import type { AppState, UiLayout } from '../types';

export class StatusBar {
  private element: HTMLElement;
  private onLayoutChange: (layout: UiLayout) => void;

  constructor(onLayoutChange: (layout: UiLayout) => void) {
    this.onLayoutChange = onLayoutChange;
    this.element = document.createElement('footer');
    this.element.className = 'status-bar';
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public render(state: AppState): void {
    const activeProvider = state.providers.find((p) => p.is_online);
    const isOnline = !!activeProvider;
    const providerName = activeProvider ? activeProvider.provider : 'Offline';
    const latency = activeProvider?.latency_ms ? `${activeProvider.latency_ms}ms` : '';
    const model = isOnline ? (state.currentModel || 'Ready') : 'No Model Detected';
    const tokRate = (isOnline && state.tokensPerSec > 0) ? `${state.tokensPerSec.toFixed(1)} tok/s` : 'Idle';
    const ram = `${state.ramUsageMb} MB (WebView2)`;

    this.element.innerHTML = `
      <div class="status-left">
        <div class="status-item brand" title="R.I.D.E Native Desktop">
          <span class="icon">⚡</span>
          <span class="label">R.I.D.E</span>
        </div>
        <div class="status-item provider-badge ${isOnline ? 'online' : 'offline'}" title="${providerName} ${latency}">
          <span class="status-dot"></span>
          <span class="text">${providerName} ${latency ? `(${latency})` : ''}</span>
        </div>
        <div class="status-item model-badge ${isOnline ? 'online' : 'offline'}" title="${isOnline ? `Active LLM: ${model}` : 'No local LLM active (Standalone Editor Mode)'}">
          <span class="icon">${isOnline ? '🧠' : '💤'}</span>
          <span class="text">${model}</span>
        </div>
        <div class="status-item tokens-badge" title="Inference Throughput">
          <span class="icon">⚡</span>
          <span class="text">${tokRate}</span>
        </div>
      </div>

      <div class="status-right">
        <div class="status-item ram-badge" title="Native WebView2 Low Memory Footprint (<60MB target)">
          <span class="icon">🍃</span>
          <span class="text">${ram}</span>
        </div>
        <div class="layout-switcher">
          <button class="layout-btn ${state.activeLayout === 'vs_code' ? 'active' : ''}" data-layout="vs_code" title="VS Code Layout (Ctrl+1)">
            <span class="icon">⊞</span> VS Code
          </button>
          <button class="layout-btn ${state.activeLayout === 'claude_code' ? 'active' : ''}" data-layout="claude_code" title="Claude Code Terminal Layout (Ctrl+2)">
            <span class="icon">❯_</span> Claude Code
          </button>
          <button class="layout-btn ${state.activeLayout === 'codex' ? 'active' : ''}" data-layout="codex" title="Codex Zen Layout (Ctrl+3)">
            <span class="icon">✧</span> Codex Zen
          </button>
        </div>
      </div>
    `;

    this.attachEvents();
  }

  private attachEvents(): void {
    const buttons = this.element.querySelectorAll<HTMLButtonElement>('.layout-btn');
    buttons.forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.preventDefault();
        const layout = btn.getAttribute('data-layout') as UiLayout;
        if (layout) {
          this.onLayoutChange(layout);
        }
      });
    });
  }
}
