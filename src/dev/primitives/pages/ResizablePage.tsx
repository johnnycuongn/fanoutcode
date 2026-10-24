import { useState, type ReactElement, type ReactNode } from "react";
import { Panel, Resizable } from "../../../components/ui";

function Pane({ children }: { children: ReactNode }): ReactElement {
  return (
    <Panel variant="card" className="grid h-full place-items-center text-sm text-mute">
      {children}
    </Panel>
  );
}

export default function ResizablePage(): ReactElement {
  const [h, setH] = useState(0.5);
  const [v, setV] = useState(2 / 3);
  const [outer, setOuter] = useState(0.4);
  const [inner, setInner] = useState(0.5);
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Resizable</h1>
      <section>
        <p className="mb-1 text-sm text-mute">Horizontal, ratio {h.toFixed(3)}</p>
        <div className="h-32">
          <Resizable
            direction="horizontal"
            ratio={h}
            onRatioChange={setH}
            first={<Pane>A</Pane>}
            second={<Pane>B</Pane>}
          />
        </div>
      </section>
      <section>
        <p className="mb-1 text-sm text-mute">Vertical, the dock at {v.toFixed(3)}</p>
        <div className="h-48">
          <Resizable
            direction="vertical"
            ratio={v}
            onRatioChange={setV}
            first={<Pane>Task page</Pane>}
            second={<Pane>Dock</Pane>}
          />
        </div>
      </section>
      <section>
        <p className="mb-1 text-sm text-mute">A nested tree mirroring a LayoutNode</p>
        <div className="h-56">
          <Resizable
            direction="horizontal"
            ratio={outer}
            onRatioChange={setOuter}
            first={<Pane>Board</Pane>}
            second={
              <Resizable
                direction="vertical"
                ratio={inner}
                onRatioChange={setInner}
                first={<Pane>Task</Pane>}
                second={<Pane>Terminal</Pane>}
              />
            }
          />
        </div>
      </section>
    </div>
  );
}
