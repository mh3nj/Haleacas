interface PlaceholderPageProps {
  title: string;
  phase: string;
}

export function PlaceholderPage({ title, phase }: PlaceholderPageProps) {
  return (
    <div className="flex h-full flex-col items-start justify-center gap-2 px-8">
      <span className="font-mono text-2xs uppercase tracking-wide text-faint">
        {phase}
      </span>
      <h1 className="text-lg font-semibold">{title}</h1>
      <p className="max-w-md text-sm text-muted">
        This workspace has not been built yet. It is on the roadmap and will
        land in a later development phase. Nothing here is faked or
        clickable in the meantime.
      </p>
    </div>
  );
}
