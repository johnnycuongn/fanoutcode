import { Plus } from "lucide-react";
import type { ReactElement } from "react";
import { Button, type ButtonVariant, type Size } from "../../../components/ui";

const variants: ButtonVariant[] = ["primary", "secondary", "ghost", "danger", "link"];
const sizes: Size[] = ["sm", "md", "lg"];

export default function ButtonPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Button</h1>
      {variants.map((variant) => (
        <section key={variant} className="flex items-center gap-3">
          <span className="w-20 text-sm text-mute">{variant}</span>
          {sizes.map((size) => (
            <Button key={size} variant={variant} size={size}>
              {size}
            </Button>
          ))}
          <Button variant={variant} loading>
            Loading
          </Button>
          <Button variant={variant} disabled>
            Disabled
          </Button>
          <Button variant={variant} leading={<Plus className="size-3.5" />} trailing={<span>⌘N</span>}>
            With icons
          </Button>
        </section>
      ))}
      <section className="flex items-center gap-3">
        <span className="w-20 text-sm text-mute">asChild</span>
        <Button asChild variant="secondary">
          <a href="#/dev/primitives/Button">A link styled as a button</a>
        </Button>
      </section>
    </div>
  );
}
