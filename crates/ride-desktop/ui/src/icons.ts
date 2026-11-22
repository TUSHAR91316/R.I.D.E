// Inline stroke icons (24×24 grid, currentColor). No emoji, no icon font —
// keeps the UI crisp at any DPI and fully offline.

const PATHS = {
  files: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><polyline points="14 3 14 8 19 8"/>',
  search: '<circle cx="11" cy="11" r="7"/><line x1="20" y1="20" x2="16.5" y2="16.5"/>',
  grid: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
  chip: '<rect x="5" y="5" width="14" height="14" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="2" x2="9" y2="5"/><line x1="15" y1="2" x2="15" y2="5"/><line x1="9" y1="19" x2="9" y2="22"/><line x1="15" y1="19" x2="15" y2="22"/><line x1="2" y1="9" x2="5" y2="9"/><line x1="2" y1="15" x2="5" y2="15"/><line x1="19" y1="9" x2="22" y2="9"/><line x1="19" y1="15" x2="22" y2="15"/>',
  sliders: '<line x1="4" y1="6" x2="20" y2="6"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="18" x2="20" y2="18"/><circle cx="9" cy="6" r="2" fill="var(--bg-chrome)"/><circle cx="15" cy="12" r="2" fill="var(--bg-chrome)"/><circle cx="7" cy="18" r="2" fill="var(--bg-chrome)"/>',
  fileAdd: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  folderOpen: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v1"/><path d="M3 17l2.5-6.5A2 2 0 0 1 7.4 9H21l-2.6 8.2A2 2 0 0 1 16.5 19H5a2 2 0 0 1-2-2z"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-2.3 5.7"/><polyline points="20 4 20 11 13 11"/>',
  collapse: '<polyline points="7 15 12 10 17 15"/>',
  chevronRight: '<polyline points="9 6 15 12 9 18"/>',
  chevronDown: '<polyline points="6 9 12 15 18 9"/>',
  close: '<line x1="6" y1="6" x2="18" y2="18"/><line x1="18" y1="6" x2="6" y2="18"/>',
  plus: '<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>',
  save: '<path d="M5 3h11l3 3v13a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M7 3v5h8V3"/><rect x="7" y="13" width="10" height="8" rx="1"/>',
  panelRight: '<rect x="3" y="4" width="18" height="16" rx="2"/><line x1="15" y1="4" x2="15" y2="20"/>',
  panelLeft: '<rect x="3" y="4" width="18" height="16" rx="2"/><line x1="9" y1="4" x2="9" y2="20"/>',
  send: '<line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="2"/>',
  check: '<polyline points="5 12 10 17 19 7"/>',
  x: '<line x1="7" y1="7" x2="17" y2="17"/><line x1="17" y1="7" x2="7" y2="17"/>',
  undo: '<polyline points="9 14 4 9 9 4"/><path d="M20 20v-5a6 6 0 0 0-6-6H4"/>',
  sparkle: '<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/>',
  terminal: '<polyline points="5 8 9 12 5 16"/><line x1="12" y1="16" x2="19" y2="16"/>',
  diff: '<line x1="6" y1="4" x2="6" y2="10"/><line x1="3" y1="7" x2="9" y2="7"/><line x1="3" y1="18" x2="9" y2="18"/><path d="M15 4h3a2 2 0 0 1 2 2v14"/><path d="M13 20h7"/>',
  command: '<path d="M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z"/>',
  layout: '<rect x="3" y="4" width="18" height="16" rx="2"/><line x1="9" y1="4" x2="9" y2="20"/><line x1="9" y1="12" x2="21" y2="12"/>',
  sun: '<circle cx="12" cy="12" r="4"/><line x1="12" y1="2" x2="12" y2="4"/><line x1="12" y1="20" x2="12" y2="22"/><line x1="4.9" y1="4.9" x2="6.3" y2="6.3"/><line x1="17.7" y1="17.7" x2="19.1" y2="19.1"/><line x1="2" y1="12" x2="4" y2="12"/><line x1="20" y1="12" x2="22" y2="12"/><line x1="4.9" y1="19.1" x2="6.3" y2="17.7"/><line x1="17.7" y1="6.3" x2="19.1" y2="4.9"/>',
  zap: '<polygon points="13 2 4 14 11 14 10 22 19 10 12 10 13 2"/>',
  copy: '<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V6a2 2 0 0 1 2-2h9"/>',
  alert: '<path d="M12 3l9.5 17h-19z"/><line x1="12" y1="10" x2="12" y2="14"/><line x1="12" y1="17" x2="12" y2="17.5"/>',
  info: '<circle cx="12" cy="12" r="9"/><line x1="12" y1="11" x2="12" y2="16"/><line x1="12" y1="8" x2="12" y2="8.5"/>',
  trash: '<polyline points="4 7 20 7"/><path d="M6 7l1 13h10l1-13"/><path d="M9 7V4h6v3"/>',
  branch: '<circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="8" r="2"/><path d="M6 7v10"/><path d="M18 10c0 4-6 3-12 7"/>',
} as const;

export type IconName = keyof typeof PATHS;

export function icon(name: IconName, extraClass = ''): string {
  return `<svg class="icon${extraClass ? ' ' + extraClass : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${PATHS[name]}</svg>`;
}

export function rideLogoSvg(size = 22, extraClass = ''): string {
  return `<svg class="ride-logo-svg${extraClass ? ' ' + extraClass : ''}" viewBox="0 0 1024 1024" width="${size}" height="${size}" fill="none" aria-hidden="true">
    <defs>
      <radialGradient id="rideBgGlow" cx="50%" cy="50%" r="65%">
        <stop offset="0%" stop-color="#141C30"/>
        <stop offset="60%" stop-color="#0A0D16"/>
        <stop offset="100%" stop-color="#05070B"/>
      </radialGradient>
      <linearGradient id="rideRustGrad" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="#FFAE00"/>
        <stop offset="35%" stop-color="#FF6B00"/>
        <stop offset="100%" stop-color="#D94100"/>
      </linearGradient>
      <linearGradient id="rideThrustGrad" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="#FF6B00"/>
        <stop offset="50%" stop-color="#FF9A22"/>
        <stop offset="100%" stop-color="#EF4444"/>
      </linearGradient>
    </defs>
    <rect x="40" y="40" width="944" height="944" rx="200" fill="url(#rideBgGlow)" stroke="#2D3C5A" stroke-width="16"/>
    <circle cx="512" cy="512" r="320" fill="#FF6B00" opacity="0.12"/>
    <circle cx="530" cy="430" r="180" fill="#00E5FF" opacity="0.16"/>
    <rect x="220" y="210" width="70" height="600" rx="14" fill="url(#rideRustGrad)"/>
    <rect x="315" y="210" width="50" height="600" rx="10" fill="url(#rideRustGrad)" opacity="0.9"/>
    <g fill="#00E5FF">
      <rect x="205" y="315" width="20" height="10" rx="4"/>
      <circle cx="190" cy="320" r="6"/>
      <line x1="196" y1="320" x2="205" y2="320" stroke="#00E5FF" stroke-width="3"/>
      <rect x="205" y="425" width="20" height="10" rx="4"/>
      <circle cx="190" cy="430" r="6"/>
      <line x1="196" y1="430" x2="205" y2="430" stroke="#00E5FF" stroke-width="3"/>
      <rect x="205" y="535" width="20" height="10" rx="4"/>
      <circle cx="190" cy="540" r="6"/>
      <line x1="196" y1="540" x2="205" y2="540" stroke="#00E5FF" stroke-width="3"/>
      <rect x="205" y="645" width="20" height="10" rx="4"/>
      <circle cx="190" cy="650" r="6"/>
      <line x1="196" y1="650" x2="205" y2="650" stroke="#00E5FF" stroke-width="3"/>
    </g>
    <path d="M 365 210 L 620 210 Q 790 210 790 390 Q 790 570 620 570 L 365 570 L 365 485 L 600 485 Q 695 485 695 390 Q 695 295 600 295 L 365 295 Z" fill="url(#rideRustGrad)"/>
    <path d="M 480 520 L 600 520 L 805 785 Q 825 810 790 810 L 710 810 L 560 610 L 460 610 Z" fill="url(#rideThrustGrad)"/>
    <path d="M 580 530 L 610 530 L 825 810 L 795 810 Z" fill="#FFD700" opacity="0.9"/>
    <g transform="translate(520, 390)">
      <circle cx="0" cy="0" r="85" fill="#00E5FF" opacity="0.3"/>
      <line x1="-140" y1="0" x2="140" y2="0" stroke="#00E5FF" stroke-width="4" opacity="0.7"/>
      <circle cx="-140" cy="0" r="6" fill="#00E5FF"/>
      <circle cx="140" cy="0" r="6" fill="#00E5FF"/>
      <polygon points="0,-75 -55,0 0,0" fill="#38BDF8"/>
      <polygon points="0,-75 55,0 0,0" fill="#7DD3FC"/>
      <polygon points="0,75 -55,0 0,0" fill="#0284C7"/>
      <polygon points="0,75 55,0 0,0" fill="#0EA5E9"/>
      <polygon points="0,-24 16,0 0,24 -16,0" fill="#FFFFFF"/>
    </g>
  </svg>`;
}
