import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// Served by GitHub Pages under the joshka.net user site, so every URL carries this prefix.
const base = '/ratatui-diff';
const canonicalBasePathScript = [
  `if (location.pathname === '${base}') {`,
  `  location.replace('${base}/' + location.search + location.hash);`,
  '}',
].join('\n');

export default defineConfig({
  site: 'https://www.joshka.net',
  base,
  trailingSlash: 'always',
  integrations: [
    starlight({
      title: 'ratatui-diff',
      description: 'A navigable unified and split diff widget for Ratatui.',
      customCss: ['./src/styles/custom.css'],
      head: [
        {
          tag: 'script',
          content: canonicalBasePathScript,
        },
      ],
      editLink: {
        baseUrl: 'https://github.com/joshka/ratatui-diff/edit/main/site/',
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/joshka/ratatui-diff',
        },
      ],
      sidebar: [
        {
          label: 'Start',
          items: [
            { label: 'Overview', link: '/' },
            { label: 'Getting started', slug: 'getting-started' },
            { label: 'Choose an input', slug: 'guides/inputs' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Appearance', slug: 'guides/appearance' },
            { label: 'Syntax colors', slug: 'guides/syntax' },
            { label: 'Navigation and search', slug: 'guides/navigation' },
            { label: 'Selection and hit-testing', slug: 'guides/selection' },
            { label: 'Folding and file headers', slug: 'guides/folding' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Interactive viewer', slug: 'reference/viewer' },
            { label: 'Performance', slug: 'reference/performance' },
            { label: 'API reference (docs.rs)', link: 'https://docs.rs/ratatui-diff' },
            { label: 'Changelog', link: 'https://github.com/joshka/ratatui-diff/blob/main/CHANGELOG.md' },
          ],
        },
      ],
    }),
  ],
});
