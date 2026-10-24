import { Archive, LayoutList } from "lucide-react";
import { useState, type ReactElement } from "react";
import { IconButton, type Size } from "../../../components/ui";

const sizes: Size[] = ["sm", "md", "lg"];

export default function IconButtonPage(): ReactElement {
  const [active, setActive] = useState(true);
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">IconButton</h1>
      {(["primary", "secondary", "ghost", "danger"] as const).map((variant) => (
        <section key={variant} className="flex items-center gap-3">
          <span className="w-20 text-sm text-mute">{variant}</span>
          {sizes.map((size) => (
            <IconButton
              key={size}
              variant={variant}
              size={size}
              label={`Archive ${size}`}
              icon={<Archive className="size-3.5" />}
            />
          ))}
        </section>
      ))}
      <section className="flex items-center gap-3">
        <span className="w-20 text-sm text-mute">toggle</span>
        <IconButton
          label="Tasks"
          icon={<LayoutList className="size-3.5" />}
          active={active}
          variant={active ? "secondary" : "ghost"}
          onClick={() => setActive((a) => !a)}
        />
      </section>
    </div>
  );
}
