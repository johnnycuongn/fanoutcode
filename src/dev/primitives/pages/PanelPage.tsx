import type { ReactElement } from "react";
import { Button, Panel, PanelBody, PanelFooter, PanelHeader } from "../../../components/ui";

export default function PanelPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Panel</h1>
      <section className="grid grid-cols-2 gap-4 bg-page">
        {(["card", "well", "bar", "plain"] as const).map((variant) => (
          <Panel key={variant} variant={variant} padding="md" aria-label={variant}>
            <PanelHeader title={variant} actions={<Button size="sm">Action</Button>} />
            <PanelBody className="text-sm text-mute">A {variant} panel.</PanelBody>
          </Panel>
        ))}
      </section>
      <Panel variant="card" className="flex h-48 flex-col" aria-label="Scrolling">
        <PanelHeader title="Scrolling body" sticky />
        <PanelBody scroll className="text-sm text-mute">
          {Array.from({ length: 20 }, (_, i) => (
            <p key={i}>Line {i + 1}</p>
          ))}
        </PanelBody>
        <PanelFooter>
          <Button size="sm" variant="primary">
            Done
          </Button>
        </PanelFooter>
      </Panel>
      <Panel variant="well" padding="md" aria-label="Nested">
        <Panel variant="card" padding="md" aria-label="Inner">
          Nested panels read the same in both themes.
        </Panel>
      </Panel>
    </div>
  );
}
