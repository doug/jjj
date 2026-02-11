const features = [
  { name: 'Works offline', jjj: true, github: false, linear: false, jira: false },
  { name: 'No server required', jjj: true, github: false, linear: false, jira: false },
  { name: 'Survives rebases', jjj: true, github: false, linear: false, jira: false },
  { name: 'Data in your repo', jjj: true, github: false, linear: false, jira: false },
  { name: 'Structured critiques', jjj: true, github: false, linear: false, jira: false },
  { name: 'AI agent native', jjj: true, github: false, linear: false, jira: false },
  { name: 'Built for Jujutsu', jjj: true, github: false, linear: false, jira: false },
  { name: 'Team collaboration', jjj: true, github: true, linear: true, jira: true },
  { name: 'Web UI', jjj: false, github: true, linear: true, jira: true },
  { name: 'Mobile app', jjj: false, github: true, linear: true, jira: true },
];

function Check() {
  return <span className="text-accent font-bold">✓</span>;
}

function Cross() {
  return <span className="text-text-secondary/50">✗</span>;
}

export function ComparisonTable() {
  return (
    <section className="py-24">
      <div className="container mx-auto px-6">
        <h2 className="text-3xl font-bold text-center text-text-primary mb-4">
          How jjj Compares
        </h2>
        <p className="text-center text-text-secondary mb-12">
          jjj vs hosted project management tools
        </p>

        <div className="overflow-x-auto">
          <table className="w-full max-w-4xl mx-auto">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left py-4 px-4 font-semibold text-text-primary">Feature</th>
                <th className="text-center py-4 px-4 font-semibold text-accent">jjj</th>
                <th className="text-center py-4 px-4 font-semibold text-text-secondary">GitHub Issues</th>
                <th className="text-center py-4 px-4 font-semibold text-text-secondary">Linear</th>
                <th className="text-center py-4 px-4 font-semibold text-text-secondary">Jira</th>
              </tr>
            </thead>
            <tbody>
              {features.map((feature) => (
                <tr key={feature.name} className="border-b border-border/50 hover:bg-surface/50">
                  <td className="py-3 px-4 text-text-primary">{feature.name}</td>
                  <td className="py-3 px-4 text-center">{feature.jjj ? <Check /> : <Cross />}</td>
                  <td className="py-3 px-4 text-center">{feature.github ? <Check /> : <Cross />}</td>
                  <td className="py-3 px-4 text-center">{feature.linear ? <Check /> : <Cross />}</td>
                  <td className="py-3 px-4 text-center">{feature.jira ? <Check /> : <Cross />}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}
