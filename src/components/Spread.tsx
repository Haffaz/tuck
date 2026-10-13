import { folderLook, tint } from "../components/look";

/** How files split across top-level folders, as one coloured bar. */
export default function Spread({ counts }: { counts: [string, number][] }) {
  const total = counts.reduce((sum, [, n]) => sum + n, 0);
  if (!total) return null;
  return (
    <div className="spread" aria-hidden="true">
      {counts
        .filter(([, n]) => n)
        .map(([folder, n]) => (
          <span
            key={folder}
            title={`${folder}: ${n}`}
            style={{ ...tint(folderLook(folder).tint), flexGrow: n }}
          />
        ))}
    </div>
  );
}
