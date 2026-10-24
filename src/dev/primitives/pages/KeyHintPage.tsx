import type { ReactElement } from "react";
import { KeyHint } from "../../../components/ui";

// The keyboard map of spec §10.5.
const MAP: [string, string][] = [
  ["Command palette", "cmd+k"],
  ["New task", "cmd+n"],
  ["New view", "cmd+t"],
  ["Close view", "cmd+w"],
  ["Next view", "ctrl+tab"],
  ["Split side by side", "cmd+d"],
  ["Split stacked", "cmd+shift+d"],
  ["Approve / advance", "cmd+enter"],
  ["Rename task", "cmd+e"],
  ["Archive task", "cmd+shift+a"],
  ["Toggle dock", "cmd+j"],
  ["Dock position", "cmd+shift+j"],
  ["Focus terminal", "ctrl+`"],
  ["Settings", "cmd+,"],
  ["Jump to view 1", "cmd+1"],
  ["Previous pane", "cmd+alt+left"],
  ["Cancel", "esc"],
];

export default function KeyHintPage(): ReactElement {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold">KeyHint</h1>
      <table className="w-80 text-sm">
        <tbody>
          {MAP.map(([action, chord]) => (
            <tr key={chord} className="border-b border-line">
              <td className="py-1.5">{action}</td>
              <td className="py-1.5 text-right">
                <KeyHint keys={chord} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="flex items-center gap-2 text-sm text-mute">
        Small: <KeyHint keys="cmd+k" size="sm" />
      </p>
    </div>
  );
}
