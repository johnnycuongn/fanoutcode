import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, defaultFilter } from "cmdk";
import type { ReactElement, ReactNode } from "react";
import { cx } from "./cx";

export type ComboboxListItem = {
  /** Unique; what `onSelect` receives. Not searched. */
  value: string;
  label: string;
  /** Extra search terms (a Jira key, a project name). */
  keywords?: string[];
  /** Items with the same group render under one heading, in first-seen order. */
  group?: string;
  hint?: ReactNode;
  disabled?: boolean;
};

export type ComboboxListProps = {
  items: readonly ComboboxListItem[];
  onSelect(value: string): void;
  /** The accessible name of the search field and the list. */
  label: string;
  placeholder?: string;
  emptyText?: string;
  renderItem?(item: ComboboxListItem): ReactNode;
  className?: string;
};

// Search the label and keywords only, so internal values never match a query.
const filter = (_value: string, search: string, keywords?: string[]) =>
  defaultFilter((keywords ?? []).join(" "), search);

/** The always-open form of Combobox: a search field over a filtered list, for the task picker and command palette.
 * Arrows move, Enter chooses the highlighted item. */
export function ComboboxList({
  items,
  onSelect,
  label,
  placeholder = "Search…",
  emptyText = "No matches",
  renderItem,
  className,
}: ComboboxListProps): ReactElement {
  const groups: Array<{ name: string | undefined; items: ComboboxListItem[] }> = [];
  for (const item of items) {
    const g = groups.find((x) => x.name === item.group);
    if (g) g.items.push(item);
    else groups.push({ name: item.group, items: [item] });
  }
  return (
    <Command label={label} filter={filter} loop className={cx("flex min-h-0 flex-col", className)}>
      <CommandInput
        autoFocus
        aria-label={label}
        placeholder={placeholder}
        className="h-9 w-full border-b border-line bg-transparent px-3 text-sm text-ink outline-none placeholder:text-mute"
      />
      <CommandList className="max-h-80 overflow-auto p-1">
        <CommandEmpty className="px-2 py-3 text-center text-sm text-mute">{emptyText}</CommandEmpty>
        {groups.map((g) => (
          <CommandGroup
            key={g.name ?? ""}
            heading={g.name}
            className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:text-mute"
          >
            {g.items.map((item) => (
              <CommandItem
                key={item.value}
                value={item.value}
                keywords={[item.label, ...(item.keywords ?? [])]}
                disabled={item.disabled}
                onSelect={() => onSelect(item.value)}
                className="flex h-8 cursor-default items-center gap-2 rounded-card px-2 text-sm text-ink data-[disabled=true]:opacity-50 data-[selected=true]:bg-well"
              >
                {renderItem ? (
                  renderItem(item)
                ) : (
                  <>
                    <span className="min-w-0 truncate">{item.label}</span>
                    {item.hint && <span className="ml-auto shrink-0 text-xs text-mute">{item.hint}</span>}
                  </>
                )}
              </CommandItem>
            ))}
          </CommandGroup>
        ))}
      </CommandList>
    </Command>
  );
}
