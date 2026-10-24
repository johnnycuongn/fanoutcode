import { Suspense, useSyncExternalStore, type ReactElement } from "react";
import { cx } from "../../components/ui";
import { pages } from "./registry";

const PREFIX = "#/dev/primitives";

function subscribe(onChange: () => void): () => void {
  window.addEventListener("hashchange", onChange);
  return () => window.removeEventListener("hashchange", onChange);
}

const readHash = () => window.location.hash;

/** Dev-only index of every primitive with its example page; route `#/dev/primitives/<Name>`. */
export function Gallery(): ReactElement {
  const hash = useSyncExternalStore(subscribe, readHash, () => "");
  const current = pages.find((p) => hash === `${PREFIX}/${p.name}`);
  const sorted = [...pages].sort((a, b) => String(a.name).localeCompare(String(b.name)));

  return (
    <div className="grid h-full grid-cols-[14rem_1fr] bg-page text-ink">
      <nav aria-label="Primitives" className="overflow-auto border-r border-line bg-bar p-2">
        <a href={PREFIX} className="mb-2 block px-2 py-1 text-sm font-medium">
          Gallery
        </a>
        <ul className="flex flex-col gap-0.5">
          {sorted.map((p) => (
            <li key={p.name}>
              <a
                href={`${PREFIX}/${p.name}`}
                aria-current={current?.name === p.name ? "page" : undefined}
                className={cx(
                  "block rounded-card px-2 py-1 text-sm hover:bg-well",
                  current?.name === p.name && "bg-well text-ink",
                )}
              >
                {p.name}
                <span className="block text-xs text-mute">{p.blurb}</span>
              </a>
            </li>
          ))}
        </ul>
      </nav>
      <main className="min-h-0 overflow-auto p-6">
        {current ? (
          <Suspense fallback={null}>
            <current.Page />
          </Suspense>
        ) : (
          <>
            <h1 className="text-lg font-semibold">Primitives gallery</h1>
            <p className="text-mute">Dev builds only. One page per primitive; add yours to registry.ts.</p>
          </>
        )}
      </main>
    </div>
  );
}
