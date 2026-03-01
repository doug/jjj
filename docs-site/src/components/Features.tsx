import { FeatureCard } from './FeatureCard';

const features = [
  {
    icon: '📡',
    title: 'Offline First',
    description: 'All metadata is stored in an orphaned git commit alongside your code. No network required — jjj works the same on a plane as in the office.',
  },
  {
    icon: '🔀',
    title: 'Survives Rebases',
    description: 'jjj tracks solutions by Jujutsu change IDs, not commit hashes. Rebase, squash, amend — your metadata links never break.',
  },
  {
    icon: '💬',
    title: 'Critique-Driven',
    description: 'Critiques block acceptance. Every concern must be explicitly addressed, validated, or dismissed — no silently ignoring feedback.',
  },
  {
    icon: '📦',
    title: 'No Server Required',
    description: 'Push jjj metadata the same way you push code: `jj git push`. Sync with teammates without any extra infrastructure.',
  },
];

export function Features() {
  return (
    <section className="py-24 bg-surface">
      <div className="container mx-auto px-6">
        <h2 className="text-3xl font-bold text-center text-text-primary mb-4">
          Why jjj?
        </h2>
        <p className="text-center text-text-secondary max-w-2xl mx-auto mb-12">
          Built for people who want project management that lives in their repository — not a SaaS dashboard.
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {features.map((feature) => (
            <FeatureCard key={feature.title} {...feature} />
          ))}
        </div>
      </div>
    </section>
  );
}
