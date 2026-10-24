// The sidebar's horizontal strip of five icon tabs.
import { FileText, GitCompare, ListTodo, Settings, Ticket } from "lucide-react";
import type { ReactElement, ReactNode } from "react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui";
import { setSidebarSection, shellStore, type SidebarSection } from "@/features/shell";

const SECTIONS: Array<{ id: SidebarSection; label: string; icon: ReactNode }> = [
  { id: "tasks", label: "Tasks", icon: <ListTodo aria-hidden className="size-4" /> },
  { id: "docs", label: "Docs", icon: <FileText aria-hidden className="size-4" /> },
  { id: "changes", label: "Changes", icon: <GitCompare aria-hidden className="size-4" /> },
  { id: "ticket", label: "Ticket", icon: <Ticket aria-hidden className="size-4" /> },
  { id: "settings", label: "Settings", icon: <Settings aria-hidden className="size-4" /> },
];

export function IconStrip(): ReactElement {
  const section = shellStore.useSelector((s) => s.sidebarSection);
  return (
    <Tabs value={section} onValueChange={(v) => setSidebarSection(v as SidebarSection)} activationMode="manual">
      <TabsList variant="pill" aria-label="Sidebar sections" className="mx-2 mt-2 flex justify-between">
        {SECTIONS.map((s) => (
          <TabsTrigger
            key={s.id}
            value={s.id}
            aria-label={s.label}
            title={s.label}
            className="flex-1 justify-center"
            onClick={() => setSidebarSection(s.id)}
          >
            {s.icon}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}
