import type { ReactElement } from "react";
import { Button, Checkbox, Combobox, Switch, cx } from "@/components/ui";
import type { Priority } from "@/lib/bindings";
import { SWATCHES, swatchClass } from "@/lib/theme";
import { clearFilters, selectJiraKeys, setFilters, useBoard } from "../store";
import type { JiraFilter } from "../types";

const PRIORITIES: Priority[] = ["High", "Medium", "Low", "None"];

const toggle = <T,>(list: T[], item: T): T[] =>
  list.includes(item) ? list.filter((x) => x !== item) : [...list, item];

function jiraValue(jira: JiraFilter): string {
  if (jira === null) return "any";
  return "key" in jira ? `key:${jira.key}` : jira.linked ? "linked" : "unlinked";
}

function jiraFilter(value: string | null): JiraFilter {
  if (value === null || value === "any") return null;
  if (value === "linked" || value === "unlinked") return { linked: value === "linked" };
  return { key: value.slice(4) };
}

/** The body of the Filter popover: priority, colour, Jira, hidden and archived. */
export function FilterPopover(): ReactElement {
  const filters = useBoard((s) => s.filters);
  const keys = useBoard(selectJiraKeys);
  const options = [
    { value: "any", label: "Any" },
    { value: "linked", label: "Linked" },
    { value: "unlinked", label: "Unlinked" },
    ...keys.map((key) => ({ value: `key:${key}`, label: key })),
  ];
  return (
    <div className="flex w-64 flex-col gap-3 p-1">
      <fieldset className="flex flex-col gap-1">
        <legend className="mb-1 text-xs text-mute">Priority</legend>
        {PRIORITIES.map((p) => (
          <Checkbox
            key={p}
            label={p}
            size="sm"
            checked={filters.priorities.includes(p)}
            onCheckedChange={() => setFilters({ priorities: toggle(filters.priorities, p) })}
          />
        ))}
      </fieldset>
      <fieldset className="flex flex-col gap-1">
        <legend className="mb-1 text-xs text-mute">Colour</legend>
        <div className="grid grid-cols-2 gap-x-2 gap-y-1">
          {SWATCHES.map((swatch) => (
            <Checkbox
              key={swatch}
              size="sm"
              checked={filters.colors.includes(swatch)}
              onCheckedChange={() => setFilters({ colors: toggle(filters.colors, swatch) })}
              label={
                <span className="inline-flex items-center gap-1.5">
                  <span aria-hidden className={cx("size-2 rounded-full", swatchClass(swatch, "bg"))} />
                  <span className="capitalize">{swatch}</span>
                </span>
              }
            />
          ))}
        </div>
      </fieldset>
      <div className="flex flex-col gap-1">
        <span className="text-xs text-mute">Jira</span>
        <Combobox
          aria-label="Jira"
          size="sm"
          options={options}
          value={jiraValue(filters.jira)}
          onValueChange={(value) => setFilters({ jira: jiraFilter(value) })}
        />
      </div>
      <Switch
        size="sm"
        label="Show hidden"
        checked={filters.showHidden}
        onCheckedChange={(showHidden) => setFilters({ showHidden })}
      />
      <Switch
        size="sm"
        label="Archived"
        checked={filters.archived}
        onCheckedChange={(archived) => setFilters({ archived })}
      />
      <Button size="sm" onClick={clearFilters} className="self-end">
        Clear
      </Button>
    </div>
  );
}
