export function QuickInstall() {
  return (
    <section className="py-16 bg-surface">
      <div className="container mx-auto px-6 text-center">
        <h2 className="text-2xl font-bold text-text-primary mb-4">
          Quick Install
        </h2>
        <p className="text-text-secondary mb-6">
          Get started in seconds with Cargo:
        </p>
        <div className="inline-flex items-center gap-3 bg-[#1E1E1E] rounded-lg px-6 py-3 font-mono text-gray-100">
          <span className="text-accent">$</span>
          <code>cargo install jjj</code>
          <button
            onClick={() => navigator.clipboard.writeText('cargo install jjj')}
            className="ml-2 text-gray-400 hover:text-accent transition-colors"
            title="Copy to clipboard"
          >
            📋
          </button>
        </div>
      </div>
    </section>
  );
}
