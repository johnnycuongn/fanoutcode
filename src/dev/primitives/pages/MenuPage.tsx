import { Archive, Flag, MoreHorizontal, Pencil, Square } from "lucide-react";
import { useState, type ReactElement } from "react";
import {
  IconButton,
  Menu,
  MenuCheckboxItem,
  MenuContent,
  MenuGroup,
  MenuItem,
  MenuLabel,
  MenuRadioGroup,
  MenuRadioItem,
  MenuSeparator,
  MenuSub,
  MenuSubContent,
  MenuSubTrigger,
  MenuTrigger,
} from "../../../components/ui";
import { SWATCHES } from "@/lib/theme";

// The card menu of spec §10.2.
export default function MenuPage(): ReactElement {
  const [priority, setPriority] = useState("medium");
  const [color, setColor] = useState("violet");
  const [flagged, setFlagged] = useState(false);
  const [hidden, setHidden] = useState(false);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold">Menu</h1>
      <Menu>
        <MenuTrigger asChild>
          <IconButton label="Card menu" icon={<MoreHorizontal className="size-4" />} variant="secondary" />
        </MenuTrigger>
        <MenuContent minWidth={240}>
          <MenuItem>Open in current view</MenuItem>
          <MenuItem hint="⌘ click">Open in new view</MenuItem>
          <MenuSub>
            <MenuSubTrigger>Open in another window</MenuSubTrigger>
            <MenuSubContent>
              <MenuItem>Window 1</MenuItem>
              <MenuItem>Window 2</MenuItem>
            </MenuSubContent>
          </MenuSub>
          <MenuSeparator />
          <MenuItem icon={<Pencil />} shortcut="cmd+e">
            Rename
          </MenuItem>
          <MenuSub>
            <MenuSubTrigger inset>Priority</MenuSubTrigger>
            <MenuSubContent>
              <MenuRadioGroup value={priority} onValueChange={setPriority}>
                {["none", "low", "medium", "high"].map((p) => (
                  <MenuRadioItem key={p} value={p}>
                    {p}
                  </MenuRadioItem>
                ))}
              </MenuRadioGroup>
            </MenuSubContent>
          </MenuSub>
          <MenuSub>
            <MenuSubTrigger inset>Colour</MenuSubTrigger>
            <MenuSubContent>
              <MenuRadioGroup value={color} onValueChange={setColor}>
                {SWATCHES.map((s) => (
                  <MenuRadioItem key={s} value={s} swatch={s}>
                    {s}
                  </MenuRadioItem>
                ))}
              </MenuRadioGroup>
            </MenuSubContent>
          </MenuSub>
          <MenuCheckboxItem checked={flagged} onCheckedChange={setFlagged}>
            Flag
          </MenuCheckboxItem>
          <MenuCheckboxItem checked={hidden} onCheckedChange={setHidden}>
            Hide from board
          </MenuCheckboxItem>
          <MenuSeparator />
          <MenuGroup>
            <MenuLabel>Session</MenuLabel>
            <MenuItem icon={<Flag />}>Link Jira issue</MenuItem>
            <MenuItem disabled hint="after the session starts">
              Toggle worktree
            </MenuItem>
            <MenuItem icon={<Square />}>Stop agent</MenuItem>
          </MenuGroup>
          <MenuSeparator />
          <MenuItem danger icon={<Archive />} shortcut="cmd+shift+a">
            Archive
          </MenuItem>
        </MenuContent>
      </Menu>
    </div>
  );
}
