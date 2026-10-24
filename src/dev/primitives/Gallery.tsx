import { Suspense, useState, useSyncExternalStore, type ReactElement } from "react";
import { Segmented, Switch, cx } from "../../components/ui";
import { pages } from "./registry";

const PREFIX = "#/dev/primitives";

function subscribe(onChange: () => void): () => void {
  window.addEventListener("hashchange", onChange);
  return () => window.removeEventListener("hashchange", onChange);
}

const readHash = () => window.location.hash;

type ThemeChoice = "light" | "dark";

function currentTheme(): ThemeChoice {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

/** Theme and density controls for looking at every page in both themes. */
function Toolbar({ dense, onDense }: { dense: boolean; onDense(dense: boolean): void }): ReactElement {
  const [theme, setTheme] = useState<ThemeChoice>(currentTheme);
  return (
    <div className="flex items-center gap-4 border-b border-line bg-bar px-6 py-2">
      <Segmented<ThemeChoice>
        aria-label="Theme"
        size="sm"
        value={theme}
        onValueChange={(next) => {
          document.documentElement.dataset.theme = next;
          setTheme(next);
        }}
        options={[
          { value: "light", label: "Light" },
          { value: "dark", label: "Dark" },
        ]}
      />
      <Switch size="sm" label="Dense" checked={dense} onCheckedChange={onDense} />
    </div>
  );
}

/** Dev-only index of every primitive with its example page; route `#/dev/primitives/<Name>`. */
export function Gallery(): ReactElement {
  const hash = useSyncExternalStore(subscribe, readHash, () => "");
  const [dense, setDense] = useState(false);
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
      <div className="flex min-h-0 flex-col">
        <Toolbar dense={dense} onDense={setDense} />
        <main className={cx("min-h-0 flex-1 overflow-auto p-6", dense && "text-xs")}>
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
    </div>
  );
}
