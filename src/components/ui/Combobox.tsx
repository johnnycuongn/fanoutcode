import { Command, CommandEmpty, CommandInput, CommandItem, CommandList } from "cmdk";
import { Check, ChevronsUpDown, X } from "lucide-react";
import { useState, type ReactElement, type ReactNode } from "react";
import { Button } from "./Button";
import { cx } from "./cx";
import { useFieldControl } from "./Field";
import { IconButton } from "./IconButton";
import { Popover, PopoverContent, PopoverTrigger } from "./Popover";

export type ComboboxOption = { value: string; label: string; hint?: string; keywords?: string[]; disabled?: boolean };

export type ComboboxProps = {
  options: readonly ComboboxOption[];
  value: string | null;
  onValueChange(value: string | null): void;
  placeholder?: string;
  searchPlaceholder?: string;
  emptyText?: string;
  size?: "sm" | "md";
  invalid?: boolean;
  disabled?: boolean;
  clearable?: boolean;
  className?: string;
  renderOption?(option: ComboboxOption, selected: boolean): ReactNode;
  "aria-label"?: string;
};

/** A searchable single choice. A `value` that matches no option shows the placeholder, never a blank trigger. */
export function Combobox({
  options,
  value,
  onValueChange,
  placeholder = "Select…",
  searchPlaceholder = "Search…",
  emptyText = "No matches",
  size = "md",
  invalid,
  disabled,
  clearable,
  className,
  renderOption,
  "aria-label": ariaLabel,
}: ComboboxProps): ReactElement {
  const [open, setOpen] = useState(false);
  const field = useFieldControl();
  const selected = options.find((o) => o.value === value);
  const choose = (next: string | null) => {
    onValueChange(next);
    setOpen(false);
  };
  return (
    <div className={cx("relative flex w-full", className)}>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            {...field}
            variant="secondary"
            size={size}
            role="combobox"
            aria-haspopup="listbox"
            aria-expanded={open}
            aria-label={ariaLabel}
            aria-invalid={invalid || field?.["aria-invalid"] ? true : undefined}
            disabled={disabled}
            className={cx(
              "w-full justify-between font-normal aria-invalid:border-bad",
              clearable && selected && "pr-8",
              !selected && "text-mute",
            )}
            trailing={<ChevronsUpDown aria-hidden className="size-3.5 shrink-0 text-mute" />}
          >
            <span className="min-w-0 truncate">{selected ? selected.label : placeholder}</span>
          </Button>
        </PopoverTrigger>
        <PopoverContent width="trigger" padding="none">
          <Command label={searchPlaceholder}>
            <CommandInput
              autoFocus
              placeholder={searchPlaceholder}
              className="h-8 w-full border-b border-line bg-transparent px-2 text-sm outline-none placeholder:text-mute"
            />
            <CommandList className="max-h-64 overflow-auto p-1">
              <CommandEmpty className="px-2 py-3 text-center text-sm text-mute">{emptyText}</CommandEmpty>
              {options.map((option) => {
                const isSelected = option.value === value;
                return (
                  <CommandItem
                    key={option.value}
                    value={option.value}
                    keywords={[option.label, ...(option.keywords ?? [])]}
                    disabled={option.disabled}
                    onSelect={() => choose(option.value)}
                    className="flex h-7 cursor-default items-center gap-2 rounded-card px-2 text-sm data-[disabled=true]:opacity-50 data-[selected=true]:bg-well"
                  >
                    <Check aria-hidden className={cx("size-3.5 shrink-0", isSelected ? "opacity-100" : "opacity-0")} />
                    {renderOption ? (
                      renderOption(option, isSelected)
                    ) : (
                      <>
                        <span className="truncate">{option.label}</span>
                        {option.hint && <span className="ml-auto truncate text-xs text-mute">{option.hint}</span>}
                      </>
                    )}
                  </CommandItem>
                );
              })}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
      {clearable && selected && !disabled && (
        <IconButton
          label="Clear"
          size="sm"
          icon={<X aria-hidden className="size-3" />}
          className="absolute top-1/2 right-6 -translate-y-1/2"
          onClick={() => onValueChange(null)}
        />
      )}
    </div>
  );
}
