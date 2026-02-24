const INSTALL_CMD = 'cargo install --git https://github.com/doug/jjj';

export function QuickInstall() {
  return (
    <section className="py-16 bg-surface">
      <div className="container mx-auto px-6 text-center">
        <h2 className="text-2xl font-bold text-text-primary mb-4">
          Get Started
        </h2>
        <p className="text-text-secondary mb-6">
          Requires Rust and <a href="https://github.com/jj-vcs/jj" className="text-accent hover:underline" target="_blank" rel="noopener noreferrer">Jujutsu</a>. Installs in seconds:
        </p>
        <div className="inline-flex items-center gap-3 bg-[#1E1E1E] rounded-lg px-6 py-3 font-mono text-gray-100">
          <span className="text-accent">$</span>
          <code>{INSTALL_CMD}</code>
          <button
            onClick={() => navigator.clipboard.writeText(INSTALL_CMD)}
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
