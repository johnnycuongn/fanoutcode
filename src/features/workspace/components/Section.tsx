// One titled block of the workspace settings screen.
import type { ReactElement, ReactNode } from "react";
import { Panel, PanelBody, PanelHeader } from "@/components/ui";

export function Section({
  title,
  actions,
  children,
}: {
  title: string;
  actions?: ReactNode;
  children: ReactNode;
}): ReactElement {
  return (
    <Panel variant="card" padding="none" aria-label={title}>
      <PanelHeader title={title} actions={actions} />
      <PanelBody className="flex flex-col gap-2 p-3">{children}</PanelBody>
    </Panel>
  );
}
