import type { ReactElement } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../../components/ui";

const DOCK = ["Memory", "Team", "Changes", "Ticket", "Test", "Commit"] as const;

function Dock({
  variant,
  orientation,
}: {
  variant: "underline" | "pill" | "bar";
  orientation?: "vertical";
}): ReactElement {
  return (
    <Tabs defaultValue="Memory" orientation={orientation} className={orientation ? "flex gap-4" : undefined}>
      <TabsList
        variant={variant}
        aria-label={`Dock ${variant}`}
        className={orientation ? "flex-col items-stretch border-b-0" : undefined}
      >
        {DOCK.map((name) => (
          <TabsTrigger key={name} value={name} count={name === "Team" ? 2 : name === "Changes" ? 5 : undefined}>
            {name}
          </TabsTrigger>
        ))}
      </TabsList>
      {DOCK.map((name) => (
        <TabsContent key={name} value={name} className="p-3 text-sm text-mute">
          {name} panel
        </TabsContent>
      ))}
    </Tabs>
  );
}

export default function TabsPage(): ReactElement {
  return (
    <div className="flex flex-col gap-8">
      <h1 className="text-lg font-semibold">Tabs</h1>
      <Dock variant="underline" />
      <Dock variant="pill" />
      <Dock variant="bar" />
      <Dock variant="underline" orientation="vertical" />
    </div>
  );
}
