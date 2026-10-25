import { useState, type ReactElement } from "react";
import { AppMark, Switch } from "../../../components/ui";

const SIZES = [16, 24, 32, 48, 96] as const;

export default function AppMarkPage(): ReactElement {
  const [spinning, setSpinning] = useState(true);
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">AppMark</h1>
      <section className="flex items-end gap-4">
        {SIZES.map((size) => (
          <span key={size} className="flex flex-col items-center gap-1 text-xs text-mute">
            <AppMark size={size} />
            {size}
          </span>
        ))}
      </section>
      <section className="flex items-center gap-4">
        <AppMark size={48} spinning={spinning} label={spinning ? "Fanout Code, agents working" : "Fanout Code"} />
        <span className="text-lg font-semibold">Fanout Code</span>
        <Switch checked={spinning} onCheckedChange={setSpinning} label="Spinning" />
      </section>
      <p className="text-sm text-mute">
        The start screen spins the mark while any agent works. It stays still under macOS Reduce motion.
      </p>
    </div>
  );
}
