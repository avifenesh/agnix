import clsx from 'clsx';
import Heading from '@theme/Heading';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import styles from './index.module.css';

const features = [
  {
    title: '100 Validation Rules',
    description:
      'Rules are sourced from knowledge-base/rules.json and rendered into searchable documentation pages.',
  },
  {
    title: 'Multi-Tool Coverage',
    description:
      'Guidance for Claude Code, AGENTS.md workflows, MCP, Cursor, GitHub Copilot, and editor integrations.',
  },
  {
    title: 'Versioned and Searchable',
    description:
      'Documentation versions are preserved, and local search indexes every page for fast rule lookup.',
  },
];

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

export default function Home() {
  return (
    <Layout title="agnix Documentation" description="agnix documentation website">
      <header className={styles.heroBanner}>
        <div className="container">
          <img src="/agnix/img/logo.png" alt="agnix" className={styles.heroLogo} />
          <Heading as="h1" className={styles.heroTitle}>
            agnix Documentation
          </Heading>
          <p className={styles.heroSubtitle}>
            Practical guides, rule reference, and editor integration docs.
          </p>
          <div>
            <Link className="button button--primary button--lg" to="/docs/getting-started">
              Open User Guide
            </Link>
          </div>
        </div>
      </header>
      <main>
        <section className={styles.features}>
          <div className="container">
            <div className="row">
              {features.map((props, idx) => (
                <Feature key={idx} {...props} />
              ))}
            </div>
          </div>
        </section>
      </main>
    </Layout>
  );
}
