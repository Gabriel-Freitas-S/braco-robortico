// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import UnoCSS from 'unocss/astro';

// O site é publicado em https://gabriel-freitas-s.github.io/braco-robortico/
// `base` faz os links e assets respeitarem o subdiretório do Pages.
export default defineConfig({
  site: 'https://gabriel-freitas-s.github.io',
  base: '/braco-robortico',
  integrations: [
    UnoCSS({ injectReset: false }),
    starlight({
      title: 'Braço Robótico 4-DOF',
      description:
        'Braço de 4 eixos impresso em PLA, com firmware Arduino e app em Rust para controle, gravação e replay de movimentos.',
      locales: {
        root: { label: 'Português', lang: 'pt-BR' },
      },
      logo: {
        src: './src/assets/logo.webp',
        alt: 'Braço Robótico 4-DOF',
      },
      favicon: '/favicon.png',
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/Gabriel-Freitas-S/braco-robortico',
        },
      ],
      sidebar: [
        { label: '← Voltar ao site', link: '/' },
        { label: 'Hardware', slug: 'hardware' },
        { label: 'Software', slug: 'software' },
        { label: 'Guia de montagem', slug: 'guia_montagem' },
        { label: 'Manual completo', link: 'https://gabriel-freitas-s.github.io/braco-robortico/manual_braco_robotico.html', attrs: { target: '_blank' } },
      ],
      customCss: ['./src/styles/starlight.css'],
      head: [
        {
          tag: 'script',
          content:
            "localStorage.setItem('starlight-theme','dark');document.documentElement.dataset.theme='dark';",
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:image',
            content: 'https://gabriel-freitas-s.github.io/braco-robortico/og.jpg',
          },
        },
        {
          tag: 'meta',
          attrs: { property: 'og:image:width', content: '1200' },
        },
        {
          tag: 'meta',
          attrs: { property: 'og:image:height', content: '630' },
        },
      ],
      editLink: {
        baseUrl:
          'https://github.com/Gabriel-Freitas-S/braco-robortico/edit/main/site/',
      },
      lastUpdated: true,
    }),
  ],
});
