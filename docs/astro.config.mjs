// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Load custom language grammars
const fshGrammar = JSON.parse(
	readFileSync(join(__dirname, 'src/lib/grammars/fsh.tmLanguage.json'), 'utf-8')
);
const gritqlGrammar = JSON.parse(
	readFileSync(join(__dirname, 'src/lib/grammars/gritql.tmLanguage.json'), 'utf-8')
);

// https://astro.build/config
export default defineConfig({
	site: 'https://octofhir.github.io',
	base: '/maki/',  // Important: subpath for GitHub Pages

	integrations: [
		starlight({
			title: 'FSH Lint',
			description: 'Fast and powerful linter for FHIR Shorthand (FSH)',

			logo: {
				src: './src/assets/logo.svg',
				replacesTitle: false,
			},

			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/octofhir/maki' },
			],

			editLink: {
				baseUrl: 'https://github.com/octofhir/maki/edit/main/docs/',
			},

			expressiveCode: {
				themes: ['github-dark', 'github-light'],
				shiki: {
					langs: [
						{ ...fshGrammar, name: 'fsh', id: 'fsh' },
						{ ...gritqlGrammar, name: 'gritql', id: 'gritql' },
					],
				},
			},

			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Introduction', slug: 'getting-started/introduction' },
						{ label: 'Installation', slug: 'getting-started/installation' },
						{ label: 'Quick Start', slug: 'getting-started/quick-start' },
					],
				},
				{
					label: 'Configuration',
					items: [
						{ label: 'Configuration File', slug: 'configuration/config-file' },
						{ label: 'Rule Configuration', slug: 'configuration/rules' },
						{ label: 'GritQL Rules', slug: 'configuration/gritql' },
						{ label: 'Schema Reference', slug: 'configuration/schema' },
					],
				},
				{
					label: 'Rules',
					// Auto-generated from devtools!
					autogenerate: { directory: 'rules' },
				},
				{
					label: 'CLI',
					items: [
						{ label: 'Commands', slug: 'cli/commands' },
						{ label: 'Autofix Reference', slug: 'cli/autofix-reference' },
						{ label: 'Options', slug: 'cli/options' },
						{ label: 'Exit Codes', slug: 'cli/exit-codes' },
					],
				},
				{
					label: 'Guides',
					items: [
						{ label: 'FSH Formatter', slug: 'guides/formatter' },
						{ label: 'Automatic Fixes', slug: 'guides/autofix' },
						{ label: 'Writing Custom Rules', slug: 'guides/custom-rules' },
						{ label: 'CI/CD Integration', slug: 'guides/ci-cd' },
						{ label: 'Editor Integration', slug: 'guides/editors' },
						{ label: 'Parent Validation', slug: 'guides/parent-validation' },
						{ label: 'Troubleshooting', slug: 'guides/troubleshooting' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'API Documentation', slug: 'reference/api' },
						{ label: 'Changelog', slug: 'reference/changelog' },
						{ label: 'Contributing', slug: 'reference/contributing' },
					],
				},
			],

			customCss: [
				'./src/styles/custom.css',
			],

			head: [
				{
					tag: 'meta',
					attrs: {
						property: 'og:image',
						content: 'https://octofhir.github.io/maki/og-image.png',
					},
				},
			],
		}),
	],
});
