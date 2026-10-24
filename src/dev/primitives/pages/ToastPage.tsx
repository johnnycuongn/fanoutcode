import type { ReactElement } from "react";
import { Button, toast } from "../../../components/ui";

export default function ToastPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Toast</h1>
      <section className="flex flex-wrap gap-3">
        <Button onClick={() => toast("Moved to Done")}>Default</Button>
        <Button onClick={() => toast.success("Plan approved")}>Success</Button>
        <Button onClick={() => toast.error("Hook failed")}>Error</Button>
        <Button onClick={() => toast.warning("Agent needs permission")}>Warning</Button>
        <Button onClick={() => toast("Task archived", { description: "Its memory folder is kept." })}>
          With description
        </Button>
        <Button onClick={() => toast("Archived", { action: { label: "Undo", onClick: () => toast("Restored") } })}>
          With Undo
        </Button>
        <Button
          onClick={() =>
            void toast.promise(new Promise((resolve) => setTimeout(resolve, 1200)), {
              loading: "Running tests…",
              success: "Tests passed",
              error: "Tests failed",
            })
          }
        >
          Promise
        </Button>
      </section>
    </div>
  );
}
