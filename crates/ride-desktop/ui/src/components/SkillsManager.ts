import type { SkillInfo } from '../types';

export class SkillsManager {
  private element: HTMLElement;
  private onToggle: (name: string, enabled: boolean) => Promise<void>;
  private filterQuery: string = '';
  private skills: SkillInfo[] = [];

  constructor(onToggle: (name: string, enabled: boolean) => Promise<void>) {
    this.onToggle = onToggle;
    this.element = document.createElement('div');
    this.element.className = 'skills-manager-container';
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public render(skills: SkillInfo[]): void {
    this.skills = skills;
    const activeCount = skills.filter((s) => s.enabled).length;

    const filtered = skills.filter((s) => {
      if (!this.filterQuery) return true;
      const q = this.filterQuery.toLowerCase();
      return (
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.triggers.some((t) => t.toLowerCase().includes(q))
      );
    });

    this.element.innerHTML = `
      <div class="skills-header">
        <div class="skills-title">
          <span class="icon">🧩</span>
          <span>Skills & Dynamic Tools</span>
          <span class="skills-badge">${activeCount} Active</span>
        </div>
        <div class="skills-search-bar">
          <input type="text" class="skills-search-input" placeholder="Filter skills or triggers..." value="${this.escapeHtml(this.filterQuery)}" />
        </div>
      </div>
      <div class="skills-list">
        ${
          filtered.length === 0
            ? `<div class="skills-empty">No skills matching "${this.escapeHtml(this.filterQuery)}"</div>`
            : filtered
                .map((skill) => {
                  return `
            <div class="skill-card ${skill.enabled ? 'active' : ''}">
              <div class="skill-card-top">
                <div class="skill-info">
                  <div class="skill-name">
                    <span class="skill-icon">⚡</span>
                    <strong>${this.escapeHtml(skill.name)}</strong>
                  </div>
                  <div class="skill-desc">${this.escapeHtml(skill.description)}</div>
                </div>
                <label class="switch" title="${skill.enabled ? 'Disable' : 'Enable'} skill">
                  <input type="checkbox" class="skill-toggle" data-skill="${this.escapeHtml(skill.name)}" ${skill.enabled ? 'checked' : ''} />
                  <span class="slider round"></span>
                </label>
              </div>
              <div class="skill-triggers">
                ${skill.triggers.map((t) => `<span class="trigger-chip">${this.escapeHtml(t)}</span>`).join('')}
              </div>
            </div>
          `;
                })
                .join('')
        }
      </div>
    `;

    this.attachEvents();
  }

  private attachEvents(): void {
    const searchInput = this.element.querySelector<HTMLInputElement>('.skills-search-input');
    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        this.filterQuery = (e.target as HTMLInputElement).value;
        this.render(this.skills);
        // refocus input
        const newSearch = this.element.querySelector<HTMLInputElement>('.skills-search-input');
        if (newSearch) {
          newSearch.focus();
          newSearch.setSelectionRange(newSearch.value.length, newSearch.value.length);
        }
      });
    }

    const toggles = this.element.querySelectorAll<HTMLInputElement>('.skill-toggle');
    toggles.forEach((toggle) => {
      toggle.addEventListener('change', async () => {
        const name = toggle.getAttribute('data-skill');
        const enabled = toggle.checked;
        if (name) {
          await this.onToggle(name, enabled);
        }
      });
    });
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.innerText = text;
    return div.innerHTML;
  }
}
