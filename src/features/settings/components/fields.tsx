// One row per setting, each writing through `setSetting(scope, key, value)` when it changes. `inherit` adds an
// Inherit choice that writes `null`, clearing the override at this scope (Plan 03).
import { useState, type ReactElement, type ReactNode } from "react";
import {
  Field,
  Input,
  Segmented,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Switch,
} from "@/components/ui";
import type { Scope } from "@/lib/bindings";
import { setSetting, useSetting } from "../store";
import type { SettingKey, Settings } from "../types";

type Base<K extends SettingKey> = { scope: Scope; settingKey: K; label: string; hint?: ReactNode; disabled?: boolean };
const INHERIT = "inherit";

export function SegmentedSetting<K extends SettingKey>({
  scope,
  settingKey,
  label,
  hint,
  options,
  inherit,
}: Base<K> & { options: Array<{ value: string; label: string }>; inherit?: boolean }): ReactElement {
  const value = useSetting(scope, settingKey) as string | null | undefined;
  const all = inherit ? [{ value: INHERIT, label: "Inherit" }, ...options] : options;
  const current = value === null || value === undefined ? (inherit ? INHERIT : "") : String(value);
  return (
    <Field label={label} hint={hint} layout="row">
      <Segmented
        aria-label={label}
        size="sm"
        options={all}
        value={current}
        onValueChange={(v) => void setSetting(scope, settingKey, (v === INHERIT ? null : v) as Settings[K])}
      />
    </Field>
  );
}

export function SwitchSetting<K extends SettingKey>({
  scope,
  settingKey,
  label,
  hint,
  disabled,
}: Base<K>): ReactElement {
  const value = useSetting(scope, settingKey);
  return (
    <Switch
      label={label}
      description={hint}
      disabled={disabled}
      checked={Boolean(value)}
      onCheckedChange={(c) => void setSetting(scope, settingKey, c as Settings[K])}
    />
  );
}

export function SelectSetting<K extends SettingKey>({
  scope,
  settingKey,
  label,
  hint,
  options,
  idOf = (v) => String(v),
}: Base<K> & {
  options: Array<{ id: string; label: string; value: Settings[K] }>;
  idOf?: (value: Settings[K]) => string;
}): ReactElement {
  const value = useSetting(scope, settingKey);
  return (
    <Field label={label} hint={hint} layout="row">
      <Select
        value={value === undefined || value === null ? undefined : idOf(value)}
        onValueChange={(id) => {
          const option = options.find((o) => o.id === id);
          if (option) void setSetting(scope, settingKey, option.value);
        }}
      >
        <SelectTrigger size="sm">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((o) => (
            <SelectItem key={o.id} value={o.id}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}

/** A text or number field, written when it loses focus or on Enter. */
export function InputSetting<K extends SettingKey>({
  scope,
  settingKey,
  label,
  hint,
  type = "text",
  min,
  max,
  mono,
  placeholder,
}: Base<K> & { type?: "text" | "number"; min?: number; max?: number; mono?: boolean; placeholder?: string }) {
  const value = useSetting(scope, settingKey);
  const [draft, setDraft] = useState<string | null>(null);
  const shown = draft ?? (value === null || value === undefined ? "" : String(value));
  const commit = () => {
    if (draft === null) return;
    setDraft(null);
    if (type === "number") {
      const n = Number(draft);
      if (!Number.isFinite(n) || (min !== undefined && n < min) || (max !== undefined && n > max)) return;
      void setSetting(scope, settingKey, n as Settings[K]);
    } else void setSetting(scope, settingKey, (draft.trim() === "" ? null : draft) as Settings[K]);
  };
  return (
    <Field label={label} hint={hint} layout="row">
      <Input
        size="sm"
        type={type}
        min={min}
        max={max}
        mono={mono}
        placeholder={placeholder}
        value={shown}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
      />
    </Field>
  );
}
