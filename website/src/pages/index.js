import React, {useEffect, useRef, useState} from 'react';
import clsx from 'clsx';
import Heading from '@theme/Heading';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import styles from './index.module.css';
import siteData from '../data/siteData.json';

const installCommand = 'npx agnix .';

const features = [
  {
    title: `${siteData.totalRules} Validation Rules`,
    description:
      `Comprehensive validation across ${siteData.categoryCount} rule categories. Catch broken skills, invalid hooks, misconfigured MCP servers, and much more.`,
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
        <polyline points="9 12 11 14 15 10" />
      </svg>
    ),
  },
  {
    title: 'Auto-Fix',
    description:
      'Fix common issues automatically. Run agnix --fix . and move on.',
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
      </svg>
    ),
  },
  {
    title: 'Editor Integration',
    description:
      'Real-time diagnostics in VS Code, Neovim, JetBrains, and Zed via the built-in LSP server.',
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="4 17 10 11 4 5" />
        <line x1="12" y1="19" x2="20" y2="19" />
      </svg>
    ),
  },
];

const tools = [
  {name: 'Claude Code', config: 'CLAUDE.md, Skills, Hooks, Agents'},
  {name: 'Cursor', config: '.cursorrules, .cursor/rules/'},
  {name: 'GitHub Copilot', config: 'copilot-instructions.md'},
  {name: 'Codex CLI', config: '.codex/config.toml, AGENTS.md'},
  {name: 'MCP', config: '*.mcp.json, JSON-RPC schemas'},
  {name: 'Cline', config: '.clinerules, .clinerules/*.md'},
  {name: 'OpenCode', config: 'opencode.json'},
  {name: 'Gemini CLI', config: 'GEMINI.md'},
  {name: 'Roo Code', config: '.roo/rules/'},
  {name: 'Kiro CLI', config: 'kiro.md'},
  {name: 'And many more', config: 'Any tool using Markdown, JSON, or YAML configs'},
];

const stats = [
  {value: String(siteData.totalRules), label: 'rules'},
  {value: '2300+', label: 'tests'},
  {value: '10+', label: 'tools'},
  {value: '5', label: 'editors'},
];

function CopyButton({text}) {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef(null);

  useEffect(() => {
    return () => clearTimeout(timeoutRef.current);
  }, []);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard access denied or unavailable
    }
  }

  return (
    <button
      className={clsx(styles.copyButton, copied && styles.copyButtonCopied)}
      onClick={handleCopy}
      aria-label={copied ? 'Copied' : 'Copy to clipboard'}
      title={copied ? 'Copied' : 'Copy to clipboard'}
    >
      {copied ? (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="20 6 9 17 4 12" />
        </svg>
      ) : (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
      )}
    </button>
  );
}

function RevealSection({children, className, ...props}) {
  const ref = useRef(null);
  // Start revealed so content is visible without JS (SSR/no-JS fallback).
  // The effect below hides it and sets up the observer on the client.
  const [revealed, setRevealed] = useState(true);

  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === 'undefined') {
      return;
    }
    // Hide on mount, then reveal via observer
    setRevealed(false);
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setRevealed(true);
          observer.disconnect();
        }
      },
      {threshold: 0.1},
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <section
      ref={ref}
      className={clsx(className, styles.reveal, revealed && styles.revealed)}
      {...props}
    >
      {children}
    </section>
  );
}

function HeroBanner() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={styles.hero}>
      <div className={styles.heroInner}>
        <div className="container">
          <img
            src="/agnix/img/logo.png"
            alt="agnix"
            className={styles.heroLogo}
            width="72"
            height="72"
          />
          <Heading as="h1" className={styles.heroTitle}>
            {siteConfig.tagline}
          </Heading>
          <p className={styles.heroSubtitle}>
            {`${siteData.totalRules} validation rules for Claude Code, Cursor, Copilot, Cline, MCP, and many more. CLI, LSP, and IDE plugins.`}
          </p>
          <div className={clsx(styles.installBlock, styles.heroInstall)}>
            <span className={styles.prompt}>$</span>
            <code>{installCommand}</code>
            <CopyButton text={installCommand} />
          </div>
          <div className={styles.heroCtas}>
            <Link
              className={clsx('button button--lg', styles.ctaPrimary)}
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
      </div>
    </header>
  );
}

function TerminalDemo() {
  /* eslint-disable react/jsx-no-comment-textnodes */
  return (
    <RevealSection className={styles.terminal}>
      <div className="container">
        <div className={styles.terminalWindow}>
          <div className={styles.terminalBar}>
            <div className={styles.terminalDots}>
              <span className={styles.terminalDot} data-color="red" />
              <span className={styles.terminalDot} data-color="yellow" />
              <span className={styles.terminalDot} data-color="green" />
            </div>
            <span className={styles.terminalTitle}>agnix</span>
          </div>
          <pre className={styles.terminalBody}><code><span className={styles.tDim}>{'$ '}</span><span className={styles.tCmd}>{'npx agnix .'}</span>{'\n'}<span className={styles.tDim}>{'Validating: .'}</span>{'\n\n'}<span className={styles.tPath}>{'CLAUDE.md:15:1'}</span>{' '}<span className={styles.tWarn}>{'warning'}</span>{": Generic instruction 'Be helpful and accurate' "}<span className={styles.tTag}>{'[fixable]'}</span>{'\n'}<span className={styles.tDim}>{'  help: Remove generic instructions. Claude already knows this.'}</span>{'\n\n'}<span className={styles.tPath}>{'.claude/skills/review/SKILL.md:3:1'}</span>{' '}<span className={styles.tErr}>{'error'}</span>{": Invalid name 'Review-Code' "}<span className={styles.tTag}>{'[fixable]'}</span>{'\n'}<span className={styles.tDim}>{"  help: Use lowercase letters and hyphens only (e.g., 'code-review')"}</span>{'\n\n'}{'Found '}<span className={styles.tErr}>{'1 error'}</span>{', '}<span className={styles.tWarn}>{'1 warning'}</span>{'\n'}<span className={styles.tDim}>{'  2 issues are automatically fixable'}</span>{'\n\n'}<span className={styles.tHint}>{'hint:'}</span>{' Run with '}<span className={styles.tCmd}>{'--fix'}</span>{' to apply fixes'}</code></pre>
        </div>
      </div>
    </RevealSection>
  );
  /* eslint-enable react/jsx-no-comment-textnodes */
}

function EditorDemo() {
  return (
    <RevealSection className={styles.editorDemo}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          See it in your editor
        </Heading>
        <div className={styles.editorDemoFrame}>
          <video
            className={styles.editorDemoGif}
            autoPlay
            loop
            muted
            playsInline
          >
            <source src="/agnix/img/demo.mp4" type="video/mp4" />
          </video>
        </div>
      </div>
    </RevealSection>
  );
}

function Feature({title, description, icon}) {
  return (
    <div className={styles.featureCard}>
      <div className={styles.featureIcon}>{icon}</div>
      <Heading as="h3" className={styles.featureTitle}>{title}</Heading>
      <p className={styles.featureDesc}>{description}</p>
    </div>
  );
}

function Features() {
  return (
    <RevealSection className={styles.features}>
      <div className="container">
        <div className={styles.featureGrid}>
          {features.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </RevealSection>
  );
}

function SupportedTools() {
  return (
    <RevealSection className={styles.tools}>
      <div className="container">
        <Heading as="h2" className={styles.sectionTitle}>
          Validates configs for
        </Heading>
        <div className={styles.toolGrid}>
          {tools.map((tool, idx) => (
            <div
              key={idx}
              className={clsx(
                styles.toolCard,
                idx === tools.length - 1 && styles.toolCardMore,
              )}
            >
              <strong>{tool.name}</strong>
              <span>{tool.config}</span>
            </div>
          ))}
        </div>
      </div>
    </RevealSection>
  );
}

function Stats() {
  return (
    <RevealSection className={styles.stats}>
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
    </RevealSection>
  );
}

function BottomCta() {
  return (
    <RevealSection className={styles.bottomCta}>
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
          className={clsx('button button--lg', styles.ctaPrimary)}
          to="/docs/getting-started"
        >
          Read the docs
        </Link>
      </div>
    </RevealSection>
  );
}

export default function Home() {
  return (
    <Layout
      title="Agent Config Linter"
      description={`Lint agent configurations before they break your workflow. ${siteData.totalRules} validation rules for Claude Code, Cursor, Copilot, Cline, MCP, and more.`}
    >
      <HeroBanner />
      <main>
        <TerminalDemo />
        <EditorDemo />
        <Features />
        <SupportedTools />
        <Stats />
        <BottomCta />
      </main>
    </Layout>
  );
}
