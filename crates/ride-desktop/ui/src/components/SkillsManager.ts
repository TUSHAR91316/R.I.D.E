import type { SkillInfo } from '../types';
import { esc } from '../util';

/** Skills registry panel. Filtering re-renders only the list, so the input keeps focus. */
export class SkillsManager {
  private element: HTMLElement;
  private listEl!: HTMLElement;
  private summaryEl!: HTMLElement;
  private onToggle: (name: string, enabled: boolean) => Promise<void>;
  private filter = '';
  private skills: SkillInfo[] = [];
  private lastRev = -1;

  constructor(onToggle: (name: string, enabled: boolean) => Promise<void>) {
    this.onToggle = onToggle;
    this.element = document.createElement('div');
    this.element.className = 'skills';
    this.element.innerHTML = `
      <div class="field">
        <label class="sr-only" for="skills-filter">Filter skills</label>
        <input id="skills-filter" type="text" placeholder="Filter skills or triggers" spellcheck="false" autocomplete="off">
      </div>
      <div class="skills-summary"></div>
      <div class="skills-list"></div>
    `;
    this.listEl = this.element.querySelector('.skills-list')!;
    this.summaryEl = this.element.querySelector('.skills-summary')!;

    this.element.querySelector('input')!.addEventListener('input', (e) => {
      this.filter = (e.target as HTMLInputElement).value;
      this.renderList();
    });
    this.listEl.addEventListener('change', (e) => {
      const input = e.target as HTMLInputElement;
      const name = input.dataset.skill;
      if (name) void this.onToggle(name, input.checked);
    });
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  public render(skills: SkillInfo[], rev: number): void {
    if (rev === this.lastRev) return;
    this.lastRev = rev;
    this.skills = skills;
    this.renderList();
  }

  private renderList(): void {
    const active = this.skills.filter((s) => s.enabled).length;
    this.summaryEl.textContent = `${active} of ${this.skills.length} enabled · loaded from ~/.ride/skills and .ride/skills`;

    const q = this.filter.trim().toLowerCase();
    const filtered = this.skills.filter(
      (s) =>
        !q ||
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.triggers.some((t) => t.toLowerCase().includes(q))
    );

    if (this.skills.length === 0) {
      this.listEl.innerHTML = `<div class="empty-note">No skills installed. Add a folder with a SKILL.md under .ride/skills/ in this workspace.</div>`;
      return;
    }
    if (filtered.length === 0) {
      this.listEl.innerHTML = `<div class="empty-note">No skills match “${esc(this.filter)}”.</div>`;
      return;
    }
    this.listEl.innerHTML = filtered
      .map(
        (s) => `<div class="skill-row${s.enabled ? '' : ' off'}">
          <div class="info">
            <span class="name">${esc(s.name)}</span>
            <span class="desc">${esc(s.description)}</span>
            ${s.triggers.length ? `<span class="triggers">${s.triggers.map((t) => `<span class="chip">${esc(t)}</span>`).join('')}</span>` : ''}
          </div>
          <label class="switch" title="${s.enabled ? 'Disable' : 'Enable'} ${esc(s.name)}">
            <span class="sr-only">Enable ${esc(s.name)}</span>
            <input type="checkbox" data-skill="${esc(s.name)}"${s.enabled ? ' checked' : ''}>
            <span class="track"></span>
          </label>
        </div>`
      )
      .join('');
  }
}
