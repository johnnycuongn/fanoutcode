// `render` inside the UiProvider (tooltips need it), with a `rerender` that keeps the provider.
import { render, type RenderResult } from "@testing-library/react";
import type { ReactElement } from "react";
import { UiProvider } from "@/components/ui";

export function mountUi(ui: ReactElement): RenderResult {
  const result = render(<UiProvider>{ui}</UiProvider>);
  return { ...result, rerender: (next) => result.rerender(<UiProvider>{next}</UiProvider>) };
}
