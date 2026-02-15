import { Button } from './ui/button';

export function Hero() {
  return (
    <section className="relative overflow-hidden py-24 lg:py-32">
      {/* Background gradient */}
      <div className="absolute inset-0 -z-10">
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] bg-gradient-radial from-accent/10 via-accent/5 to-transparent rounded-full blur-3xl" />
      </div>

      <div className="container mx-auto px-6 text-center">
        {/* Logo placeholder */}
        <div className="mb-8">
          <span className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-accent text-white text-2xl font-bold">
            jjj
          </span>
        </div>

        {/* Headline */}
        <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold text-text-primary tracking-tight mb-6">
          Distributed Project Management
          <br />
          <span className="text-accent">for Jujutsu</span>
        </h1>

        {/* Subtitle */}
        <p className="text-xl text-text-secondary max-w-3xl mx-auto mb-10">
          The first project management tool built on <strong>Critical Rationalism</strong>.
          <br />
          Problems, conjectures, and critiques — fully distributed, offline-first, and anchored to your code.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <a href="/getting-started/installation">
            <Button size="lg">Get Started</Button>
          </a>
          <a href="https://github.com/doug/jjj" target="_blank" rel="noopener noreferrer">
            <Button variant="outline" size="lg">
              View on GitHub
            </Button>
          </a>
        </div>
      </div>
    </section>
  );
}
