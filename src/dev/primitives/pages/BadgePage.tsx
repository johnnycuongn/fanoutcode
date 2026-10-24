import type { ReactElement } from "react";
import { Badge, STAGES, type Tone } from "../../../components/ui";
import { SWATCHES } from "@/lib/theme";

const tones: Tone[] = ["neutral", "accent", "ok", "warn", "bad"];

export default function BadgePage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Badge</h1>
      {(["soft", "solid", "outline"] as const).map((variant) => (
        <section key={variant} className="flex items-center gap-2">
          <span className="w-16 text-sm text-mute">{variant}</span>
          {tones.map((tone) => (
            <Badge key={tone} tone={tone} variant={variant}>
              {tone}
            </Badge>
          ))}
          <Badge tone="accent" variant={variant} size="sm">
            small
          </Badge>
        </section>
      ))}
      <section className="flex flex-wrap items-center gap-2">
        <span className="w-16 text-sm text-mute">stages</span>
        {STAGES.map((stage) => (
          <Badge key={stage} stage={stage}>
            {stage}
          </Badge>
        ))}
      </section>
      <section className="flex flex-wrap items-center gap-2">
        <span className="w-16 text-sm text-mute">swatches</span>
        {SWATCHES.map((swatch) => (
          <Badge key={swatch} swatch={swatch} variant="solid">
            {swatch}
          </Badge>
        ))}
      </section>
      <section className="flex items-center gap-2">
        <span className="w-16 text-sm text-mute">mono</span>
        <Badge mono tone="accent">
          TC-42
        </Badge>
        <Badge dot tone="ok">
          with dot
        </Badge>
      </section>
      <section className="flex items-center gap-2">
        <span className="w-16 text-sm text-mute">long</span>
        <div className="w-72">
          <Badge tone="warn">
            Replied: I could not find the test command; which script should I run for the integration suite in this
            repository?
          </Badge>
        </div>
      </section>
    </div>
  );
}
