import type { ReactElement } from "react";
import {
  Button,
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
  Field,
  Input,
} from "../../../components/ui";

function Example({ side, size }: { side: "bottom" | "right"; size: "sm" | "md" | "lg" }): ReactElement {
  return (
    <Drawer side={side}>
      <DrawerTrigger asChild>
        <Button>
          {side} {size}
        </Button>
      </DrawerTrigger>
      <DrawerContent size={size}>
        <DrawerHeader>
          <DrawerTitle>Costs</DrawerTitle>
          <DrawerDescription>Spend by task, stage and model.</DrawerDescription>
        </DrawerHeader>
        <form className="flex flex-col gap-3 px-4" onSubmit={(e) => e.preventDefault()}>
          <Field label="Budget">
            <Input defaultValue="25.00" mono />
          </Field>
        </form>
        <DrawerFooter>
          <DrawerClose asChild>
            <Button>Close</Button>
          </DrawerClose>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}

export default function DrawerPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Drawer</h1>
      <section className="flex flex-wrap gap-3">
        {(["sm", "md", "lg"] as const).map((size) => (
          <Example key={`b${size}`} side="bottom" size={size} />
        ))}
        {(["sm", "md", "lg"] as const).map((size) => (
          <Example key={`r${size}`} side="right" size={size} />
        ))}
      </section>
    </div>
  );
}
