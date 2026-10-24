// App settings: appearance, terminal, layout and model provider.
import type { ReactElement } from "react";
import { ACCENTS, DETAIL_LEVELS, MODEL_PROVIDERS, TERMINAL_THEMES, THEMES, appScope, terminalThemeId } from "../store";
import { InputSetting, SegmentedSetting, SelectSetting, SwitchSetting } from "./fields";

const opts = (values: readonly string[]) => values.map((v) => ({ value: v, label: v }));

export function AppearanceFields(): ReactElement {
  const scope = appScope();
  return (
    <>
      <SegmentedSetting scope={scope} settingKey="theme" label="Theme" options={opts(THEMES)} />
      <SegmentedSetting scope={scope} settingKey="accent" label="Accent" options={opts(ACCENTS)} />
      <SelectSetting
        scope={scope}
        settingKey="terminal_theme"
        label="Terminal theme"
        options={TERMINAL_THEMES}
        idOf={terminalThemeId}
      />
      <InputSetting scope={scope} settingKey="terminal_font_family" label="Terminal font" />
      <InputSetting
        scope={scope}
        settingKey="terminal_font_size"
        label="Terminal font size"
        type="number"
        min={9}
        max={32}
      />
      <SwitchSetting scope={scope} settingKey="terminal_ligatures" label="Ligatures" />
      <SegmentedSetting scope={scope} settingKey="detail_level" label="Detail level" options={opts(DETAIL_LEVELS)} />
      <SegmentedSetting
        scope={scope}
        settingKey="dock_position"
        label="Dock position"
        options={[
          { value: "Bottom", label: "Bottom" },
          { value: "Right", label: "Right inspector" },
        ]}
      />
      <SelectSetting
        scope={scope}
        settingKey="model_provider"
        label="Model provider"
        options={MODEL_PROVIDERS.map((p) => ({ id: p.value, label: p.label, value: p.value }))}
      />
    </>
  );
}
