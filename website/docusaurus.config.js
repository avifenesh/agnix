// @ts-check

const config = {
  title: 'agnix',
  tagline: 'Lint agent configurations before they break your workflow',
  favicon: 'img/logo.png',

  url: 'https://avifenesh.github.io',
  baseUrl: '/agnix/',

  organizationName: 'avifenesh',
  projectName: 'agnix',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  stylesheets: [
    {
      href: 'https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap',
      type: 'text/css',
    },
  ],

  headTags: [
    {
      tagName: 'script',
      attributes: { type: 'application/ld+json' },
      innerHTML: JSON.stringify({
        '@context': 'https://schema.org',
        '@type': 'SoftwareApplication',
        name: 'agnix',
        description:
          'Linter for AI agent configurations. 145 validation rules for Skills, Hooks, MCP, Memory, and Plugins across Claude Code, Copilot, Cursor, Cline, and more.',
        applicationCategory: 'DeveloperApplication',
        operatingSystem: 'Windows, macOS, Linux',
        url: 'https://avifenesh.github.io/agnix/',
        downloadUrl: 'https://github.com/avifenesh/agnix/releases',
        codeRepository: 'https://github.com/avifenesh/agnix',
        programmingLanguage: 'Rust',
        license: 'https://opensource.org/licenses/MIT',
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
      }),
    },
  ],

  presets: [
    [
      'classic',
      {
        docs: {
          path: 'docs',
          routeBasePath: 'docs',
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl: 'https://github.com/avifenesh/agnix/tree/main/website/',
          showLastUpdateTime: true,
          lastVersion: 'current',
          versions: {
            current: {
              label: 'next',
            },
          },
        },
        sitemap: {
          changefreq: 'weekly',
          priority: 0.5,
          lastmod: 'date',
          filename: 'sitemap.xml',
        },
        blog: false,
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],

  plugins: [
    [
      require.resolve('@easyops-cn/docusaurus-search-local'),
      {
        indexDocs: true,
        docsRouteBasePath: '/docs',
        language: ['en'],
        hashed: true,
        highlightSearchTermsOnTargetPage: true,
      },
    ],
  ],

  themeConfig: {
    image: 'img/logo.png',

    colorMode: {
      defaultMode: 'light',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },

    metadata: [
      {
        name: 'keywords',
        content:
          'agent config linter, AGENTS.md validator, Claude Code linter, MCP validation, CLAUDE.md linter, agnix, cursor rules linter, cline linter, opencode validator, gemini cli linter',
      },
      { name: 'twitter:card', content: 'summary_large_image' },
      { property: 'og:type', content: 'website' },
      { property: 'og:site_name', content: 'agnix' },
    ],

    navbar: {
      hideOnScroll: true,
      title: 'agnix',
      logo: {
        alt: 'agnix logo',
        src: 'img/logo.png',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          to: '/docs/rules',
          label: 'Rules',
          position: 'left',
        },
        {
          type: 'docsVersionDropdown',
          position: 'left',
          dropdownActiveClassDisabled: true,
        },
        {
          label: 'Editors',
          position: 'right',
          items: [
            {
              label: 'VS Code',
              href: 'https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix',
            },
            {
              label: 'JetBrains',
              href: 'https://plugins.jetbrains.com/plugin/30087-agnix',
            },
            {
              label: 'Neovim',
              href: 'https://github.com/avifenesh/agnix/tree/main/editors/neovim',
            },
            {
              label: 'Zed',
              href: 'https://github.com/avifenesh/agnix/tree/main/editors/zed',
            },
          ],
        },
        {
          href: 'https://github.com/avifenesh/agnix',
          position: 'right',
          className: 'header-github-link',
          'aria-label': 'GitHub repository',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            { label: 'Getting Started', to: '/docs/getting-started' },
            { label: 'Installation', to: '/docs/installation' },
            { label: 'Configuration', to: '/docs/configuration' },
            { label: 'Rules Reference', to: '/docs/rules' },
          ],
        },
        {
          title: 'Editors',
          items: [
            { label: 'VS Code', href: 'https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix' },
            { label: 'JetBrains', href: 'https://plugins.jetbrains.com/plugin/30087-agnix' },
            { label: 'Neovim', to: '/docs/editor-integration' },
            { label: 'Zed', to: '/docs/editor-integration' },
          ],
        },
        {
          title: 'Community',
          items: [
            { label: 'GitHub', href: 'https://github.com/avifenesh/agnix' },
            { label: 'Issues', href: 'https://github.com/avifenesh/agnix/issues' },
            { label: 'Discussions', href: 'https://github.com/avifenesh/agnix/discussions' },
          ],
        },
        {
          title: 'More',
          items: [
            { label: 'npm', href: 'https://www.npmjs.com/package/agnix' },
            { label: 'crates.io', href: 'https://crates.io/crates/agnix-cli' },
            { label: 'Releases', href: 'https://github.com/avifenesh/agnix/releases' },
          ],
        },
      ],
      copyright: `Copyright \u00A9 ${new Date().getFullYear()} agnix contributors. MIT / Apache-2.0`,
    },
    prism: {
      additionalLanguages: ['toml', 'json', 'bash', 'yaml', 'rust'],
    },
  },
};

module.exports = config;
