import { Button } from './ui/button';

export function Hero({ base = '/' }: { base?: string }) {
  return (
    <section className="relative overflow-hidden py-24 lg:py-32">
      {/* Background gradient */}
      <div className="absolute inset-0 -z-10">
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] bg-gradient-radial from-accent/10 via-accent/5 to-transparent rounded-full blur-3xl" />
      </div>

      <div className="container mx-auto px-6 text-center">
        {/* Badge */}
        <div className="mb-8 flex items-center justify-center gap-3">
          <span className="inline-flex items-center justify-center w-12 h-12 rounded-xl bg-accent text-white text-lg font-black shadow-lg shadow-accent/20">
            jjj
          </span>
          <span className="text-text-secondary font-medium tracking-wide">Jujutsu Juggler</span>
        </div>

        {/* Headline */}
        <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold text-text-primary tracking-tight mb-6">
          Distributed Project Management
          <br />
          <span className="text-accent text-3xl md:text-4xl lg:text-5xl">for Jujutsu</span>
        </h1>

        {/* Subtitle */}
        <p className="text-xl text-text-secondary max-w-3xl mx-auto mb-10">
          Problems → Solutions → Critiques. Offline-first, no server, no database.
          <br />
          Metadata lives in your repo and survives every rebase.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row gap-4 justify-center mb-12">
          <a href={`${base}getting-started/installation`}>
            <Button size="lg">Get Started</Button>
          </a>
          <a href="https://github.com/doug/jjj" target="_blank" rel="noopener noreferrer">
            <Button variant="outline" size="lg">
              View on GitHub
            </Button>
          </a>
        </div>

        {/* Demo GIF */}
        <div className="max-w-3xl mx-auto rounded-xl overflow-hidden shadow-2xl shadow-black/30">
          <img src="/demo/workflow.gif" alt="jjj workflow demo — creating problems, solutions, critiques" className="w-full" />
        </div>
      </div>
    </section>
  );
}
