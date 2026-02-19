import { Button } from './ui/button';

export function FooterCTA({ base = '/' }: { base?: string }) {
  return (
    <section className="py-24 bg-gradient-to-br from-surface to-accent/5">
      <div className="container mx-auto px-6 text-center">
        <h2 className="text-3xl font-bold text-text-primary mb-4">
          Ready to start?
        </h2>
        <p className="text-text-secondary max-w-md mx-auto mb-8">
          Join the developers using jjj to manage projects with intellectual honesty.
        </p>
        <a href={`${base}getting-started/installation`}>
          <Button size="lg">Read the Docs</Button>
        </a>
      </div>
    </section>
  );
}
