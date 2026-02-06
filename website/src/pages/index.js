import clsx from 'clsx';
import Heading from '@theme/Heading';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import styles from './index.module.css';

const installCommand = 'npx agnix .';

const terminalOutput = `$ npx agnix .
Validating: .

CLAUDE.md:15:1 warning: Generic instruction 'Be helpful and accurate' [fixable]
  help: Remove generic instructions. Claude already knows this.

.claude/skills/review/SKILL.md:3:1 error: Invalid name 'Review-Code' [fixable]
  help: Use lowercase letters and hyphens only (e.g., 'code-review')

Found 1 error, 1 warning
  2 issues are automatically fixable

hint: Run with --fix to apply fixes`;

const features = [
  {
    title: '100 Validation Rules',
    description:
      'Catch broken skills, invalid hooks, misconfigured MCP servers, and 97 other issues across 11 config formats.',
  },
  {
    title: 'Auto-Fix',
    description:
      'Fix common issues automatically. Run agnix --fix . and move on.',
  },
  {
    title: 'Editor Integration',
    description:
      'Real-time diagnostics in VS Code, Neovim, JetBrains, and Zed via the built-in LSP server.',
  },
];

const tools = [
  { name: 'Claude Code', category: 'CLAUDE.md, Skills, Hooks, Agents, Plugins' },
  { name: 'Cursor', category: '.cursorrules, .cursor/rules/' },
  { name: 'GitHub Copilot', category: 'copilot-instructions.md' },
  { name: 'Codex CLI / AGENTS.md', category: 'AGENTS.md, AGENTS.local.md' },
  { name: 'MCP', category: '*.mcp.json, JSON-RPC schemas' },
];

const stats = [
  { value: '100', label: 'rules' },
  { value: '1600+', label: 'tests' },
  { value: '5', label: 'editors' },
  { value: '3', label: 'output formats' },
];

function CopyButton({ text }) {
  return (
    <button
      className={styles.copyButton}
      onClick={() => {
        navigator.clipboard.writeText(text);
      }}
      aria-label="Copy to clipboard"
      title="Copy to clipboard"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
      </svg>
    </button>
  );
}

function HeroBanner() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={styles.hero}>
      <div className="container">
        <img
          src="/agnix/img/logo.png"
          alt="agnix"
          className={styles.heroLogo}
          width="80"
          height="80"
        />
        <Heading as="h1" className={styles.heroTitle}>
          {siteConfig.tagline}
        </Heading>
        <p className={styles.heroSubtitle}>
          100 validation rules for Claude Code, Cursor, Copilot, MCP, and
          AGENTS.md configs. CLI, LSP, and IDE plugins.
        </p>
        <div className={styles.installBlock}>
          <span className={styles.prompt}>$</span>
          <code>{installCommand}</code>
          <CopyButton text={installCommand} />
        </div>
        <div className={styles.heroCtas}>
          <Link
            className={clsx('button button--primary button--lg', styles.ctaPrimary)}
            to="/docs/getting-started"
          >
            Get Started
          </Link>
          <Link
            className={clsx('button button--lg', styles.ctaGithub)}
            href="https://github.com/avifenesh/agnix"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style={{marginRight: '0.5rem', verticalAlign: 'text-bottom'}}>
              <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
            </svg>
            GitHub
          </Link>
        </div>
        <div className={styles.badges}>
          <a href="https://github.com/avifenesh/agnix/actions/workflows/ci.yml">
            <img src="https://github.com/avifenesh/agnix/actions/workflows/ci.yml/badge.svg" alt="CI" />
          </a>
          <a href="https://www.npmjs.com/package/agnix">
            <img src="https://img.shields.io/npm/v/agnix.svg" alt="npm" />
          </a>
          <a href="https://crates.io/crates/agnix-cli">
            <img src="https://img.shields.io/crates/v/agnix-cli.svg" alt="crates.io" />
          </a>
          <a href="https://github.com/avifenesh/agnix/blob/main/LICENSE-MIT">
            <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License" />
          </a>
        </div>
        <p className={styles.starNudge}>
          Like what you see?{' '}
          <a href="https://github.com/avifenesh/agnix/stargazers">
            Give it a star
          </a>
          {' '}&mdash; it helps other developers find agnix.
        </p>
      </div>
    </header>
  );
}

function TerminalDemo() {
  return (
    <section className={styles.terminal}>
      <div className="container">
        <div className={styles.terminalWindow}>
          <div className={styles.terminalBar}>
            <span className={styles.terminalDot} />
            <span className={styles.terminalDot} />
            <span className={styles.terminalDot} />
          </div>
          <pre className={styles.terminalBody}>
            <code>{terminalOutput}</code>
          </pre>
        </div>
      </div>
    </section>
  );
}

function Feature({ title, description }) {
  return (
    <div className={clsx('col col--4')}>
      <div className={styles.featureCard}>
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

function Features() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {features.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}

function SupportedTools() {
  return (
    <section className={styles.tools}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          Validates configs for
        </Heading>
        <div className={styles.toolGrid}>
          {tools.map((tool, idx) => (
            <div key={idx} className={styles.toolCard}>
              <strong>{tool.name}</strong>
              <span>{tool.category}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function Stats() {
  return (
    <section className={styles.stats}>
      <div className="container">
        <div className={styles.statsRow}>
          {stats.map((stat, idx) => (
            <div key={idx} className={styles.statItem}>
              <span className={styles.statValue}>{stat.value}</span>
              <span className={styles.statLabel}>{stat.label}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function BottomCta() {
  return (
    <section className={styles.bottomCta}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          Try it now
        </Heading>
        <div className={styles.installBlock}>
          <span className={styles.prompt}>$</span>
          <code>{installCommand}</code>
          <CopyButton text={installCommand} />
        </div>
        <p className={styles.bottomCtaText}>
          Zero install, zero config. Finds real issues in seconds.
        </p>
        <Link
          className="button button--primary button--lg"
          to="/docs/getting-started"
        >
          Read the docs
        </Link>
      </div>
    </section>
  );
}

export default function Home() {
  return (
    <Layout
      title="Agent Config Linter"
      description="Lint agent configurations before they break your workflow. 100 validation rules for Claude Code, Cursor, Copilot, MCP, and AGENTS.md."
    >
      <HeroBanner />
      <main>
        <TerminalDemo />
        <Features />
        <SupportedTools />
        <Stats />
        <BottomCta />
      </main>
    </Layout>
  );
}
