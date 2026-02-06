// @ts-check

const config = {
  title: 'agnix Documentation',
  tagline: 'Validate agent configuration with confidence',
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

  presets: [
    [
      'classic',
      {
        docs: {
          path: 'docs',
          routeBasePath: 'docs',
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl: 'https://github.com/avifenesh/agnix/tree/main/website/',
          lastVersion: 'current',
          versions: {
            current: {
              label: 'next',
            },
          },
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
    navbar: {
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
          label: 'Documentation',
        },
        {
          type: 'docsVersionDropdown',
          position: 'left',
          dropdownActiveClassDisabled: true,
        },
        {
          href: 'https://github.com/avifenesh/agnix',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Documentation',
          items: [
            {
              label: 'Getting Started',
              to: '/docs/getting-started',
            },
            {
              label: 'Rules Reference',
              to: '/docs/rules',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub Issues',
              href: 'https://github.com/avifenesh/agnix/issues',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} agnix contributors.`,
    },
    prism: {
      additionalLanguages: ['toml', 'json', 'bash'],
    },
  },
};

module.exports = config;
