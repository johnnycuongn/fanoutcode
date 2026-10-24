import { ListFilter } from "lucide-react";
import type { ReactElement } from "react";
import { Badge, Button, Chip, Popover, PopoverContent, PopoverTrigger } from "@/components/ui";
import { countActiveFilters } from "../filters";
import { setFilters, useBoard } from "../store";
import { FilterPopover } from "./FilterPopover";

export function FilterBar(): ReactElement {
  const needsYou = useBoard((s) => s.filters.needsYou);
  const active = useBoard((s) => countActiveFilters(s.filters));
  return (
    <div className="flex items-center gap-2">
      <Chip selected={needsYou} tone="warn" onClick={() => setFilters({ needsYou: !needsYou })}>
        Needs you
      </Chip>
      <Popover>
        <PopoverTrigger asChild>
          <Button size="sm" leading={<ListFilter aria-hidden className="size-3.5" />}>
            Filter
            {active > 0 && (
              <Badge size="sm" tone="accent" variant="solid">
                {active}
              </Badge>
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent>
          <FilterPopover />
        </PopoverContent>
      </Popover>
    </div>
  );
}
