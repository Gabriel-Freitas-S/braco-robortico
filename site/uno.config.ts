import { defineConfig, presetIcons, presetWind3 } from 'unocss';

// Cores e atalhos usados na landing e nos componentes das páginas de docs.
// O CSS de layout continua em src/styles; o UnoCSS cobre utilitários e ícones.
export default defineConfig({
  // `public/` tem HTML estático (manual, rustdoc) que não usa utilitários;
  // varrê-lo só gerava classes-lixo — inclusive uma `.where` animada que
  // colidia com o CSS da landing.
  content: {
    pipeline: {
      exclude: ['public/**', 'dist/**'],
    },
  },
  presets: [
    presetWind3(),
    presetIcons({
      scale: 1.15,
      warn: true,
      extraProperties: {
        display: 'inline-block',
        'vertical-align': 'middle',
      },
    }),
  ],
  theme: {
    colors: {
      ink: '#14181d',
      raised: '#1b2027',
      line: '#2a3139',
      fg: '#e8edf2',
      muted: '#9aa7b4',
      accent: '#7fb2e5',
      'accent-strong': '#a8cdf0',
      ok: '#8fd6a2',
      warn: '#d9c376',
    },
  },
  shortcuts: {
    'btn-base':
      'inline-flex items-center gap-2 px-5 py-3 rounded-lg font-600 text-sm no-underline transition-colors',
    'btn-primary': 'btn-base bg-accent text-[#0c1116] hover:bg-accent-strong',
    'btn-ghost':
      'btn-base border border-line text-fg hover:bg-raised hover:border-[#3d4855]',
    'chip':
      'inline-flex items-center gap-2 font-mono text-[.72rem] tracking-wide uppercase text-accent',
  },
});
