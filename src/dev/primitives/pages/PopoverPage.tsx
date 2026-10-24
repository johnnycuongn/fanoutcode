import type { ReactElement } from "react";
import { Button, Field, Input, Popover, PopoverClose, PopoverContent, PopoverTrigger } from "../../../components/ui";

export default function PopoverPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Popover</h1>
      <section className="flex items-center gap-3">
        <Popover>
          <PopoverTrigger asChild>
            <Button>Auto width</Button>
          </PopoverTrigger>
          <PopoverContent>Sized to its content.</PopoverContent>
        </Popover>
        <Popover>
          <PopoverTrigger asChild>
            <Button className="w-64">Trigger width</Button>
          </PopoverTrigger>
          <PopoverContent width="trigger">Same width as the trigger.</PopoverContent>
        </Popover>
        {(["none", "sm", "md"] as const).map((padding) => (
          <Popover key={padding}>
            <PopoverTrigger asChild>
              <Button variant="ghost">padding {padding}</Button>
            </PopoverTrigger>
            <PopoverContent padding={padding}>Padding {padding}</PopoverContent>
          </Popover>
        ))}
      </section>
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="primary">With a form</Button>
        </PopoverTrigger>
        <PopoverContent padding="md" className="w-72">
          <form className="flex flex-col gap-3" onSubmit={(e) => e.preventDefault()}>
            <Field label="Rename task">
              <Input defaultValue="Add rate limiting" />
            </Field>
            <PopoverClose asChild>
              <Button variant="primary" type="submit">
                Save
              </Button>
            </PopoverClose>
          </form>
        </PopoverContent>
      </Popover>
    </div>
  );
}
