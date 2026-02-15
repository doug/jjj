import { FeatureCard } from './FeatureCard';

const features = [
  {
    icon: '📡',
    title: 'Offline First',
    description: 'All metadata lives in your repo. Works on a plane.',
  },
  {
    icon: '🔀',
    title: 'Survives Rebases',
    description: 'Change IDs persist across history rewrites.',
  },
  {
    icon: '💬',
    title: 'Critique-Driven',
    description: 'Solutions must survive criticism before acceptance.',
  },
  {
    icon: '📦',
    title: 'No Server Required',
    description: 'Sync metadata as easily as you sync code.',
  },
];

export function Features() {
  return (
    <section className="py-24 bg-surface">
      <div className="container mx-auto px-6">
        <h2 className="text-3xl font-bold text-center text-text-primary mb-12">
          Why jjj?
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {features.map((feature) => (
            <FeatureCard key={feature.title} {...feature} />
          ))}
        </div>
      </div>
    </section>
  );
}
