import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: process.env.DOCS_SITE,
  base: process.env.DOCS_BASE ?? '/',
  integrations: [starlight({
    title: 'Pixerve',
    logo: { src: '../../branding/pixerve-logo.svg', alt: 'Pixerve', replacesTitle: true },
    description: 'Install Pixerve, resize images, and connect your CMS.',
    favicon: '/favicon.ico',
    social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/xviidev/pixerve' }],
    customCss: ['./src/styles/custom.css'],
    sidebar: [
      { label: 'Start here', items: [
        { label: 'Overview', slug: '' },
        { label: 'Installation', slug: 'start/installation' },
        { label: 'Use Pixerve', slug: 'start/usage' },
      ] },
      { label: 'Guides', items: [
        { label: 'CMS integration', slug: 'guides/cms' },
        { label: 'Signing and access', slug: 'guides/signing' },
        { label: 'Caching and delivery', slug: 'guides/delivery' },
        { label: 'Deployment', slug: 'guides/deployment' },
        { label: 'Operations', slug: 'guides/operations' },
        { label: 'Static files and social images', slug: 'guides/assets' },
      ] },
      { label: 'Reference', items: [
        { label: 'JavaScript SDK', slug: 'reference/sdk' },
        { label: 'HTTP API', slug: 'reference/http' },
        { label: 'Configuration', slug: 'reference/configuration' },
        { label: 'CLI and signing protocol', slug: 'reference/cli' },
      ] },
      { label: 'Contributing', collapsed: true, items: [
        { label: 'Development', slug: 'contributing/development' },
        { label: 'Architecture', slug: 'contributing/architecture' },
      ] },
    ],
  })],
});
