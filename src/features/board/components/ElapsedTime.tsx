import type { ReactElement } from "react";
import { useElapsed } from "../elapsed";

/** The only board component that subscribes to the clock. */
export function ElapsedTime({ since }: { since: string }): ReactElement {
  const text = useElapsed(since);
  return (
    <time dateTime={since} data-testid="elapsed" className="text-mute tabular-nums">
      {text}
    </time>
  );
}
