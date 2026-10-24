import type { ReactElement } from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  Field,
  Input,
  Textarea,
} from "../../../components/ui";

export default function DialogPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Dialog</h1>
      <section className="flex flex-wrap items-center gap-3">
        {(["sm", "md", "lg"] as const).map((size) => (
          <Dialog key={size}>
            <DialogTrigger asChild>
              <Button>Size {size}</Button>
            </DialogTrigger>
            <DialogContent size={size}>
              <DialogHeader>
                <DialogTitle>Size {size}</DialogTitle>
                <DialogDescription>A dialog at the {size} width.</DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose asChild>
                  <Button>Close</Button>
                </DialogClose>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        ))}
        <Dialog>
          <DialogTrigger asChild>
            <Button variant="danger">Archive…</Button>
          </DialogTrigger>
          <DialogContent size="sm">
            <DialogHeader>
              <DialogTitle>Archive task?</DialogTitle>
              <DialogDescription>The agent is still working. Archiving stops it.</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <DialogClose asChild>
                <Button>Cancel</Button>
              </DialogClose>
              <DialogClose asChild>
                <Button variant="danger">Archive</Button>
              </DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>
        <Dialog>
          <DialogTrigger asChild>
            <Button variant="primary">New task</Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>New task</DialogTitle>
              <DialogDescription>Describe what you want done.</DialogDescription>
            </DialogHeader>
            <form className="flex flex-col gap-3" onSubmit={(e) => e.preventDefault()}>
              <Field label="Title">
                <Input />
              </Field>
              <Field label="Intent" required>
                <Textarea rows={4} />
              </Field>
              <DialogFooter>
                <DialogClose asChild>
                  <Button>Cancel</Button>
                </DialogClose>
                <Button variant="primary" type="submit">
                  Create
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
        <Dialog>
          <DialogTrigger asChild>
            <Button>Nested</Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Settings</DialogTitle>
              <DialogDescription>Escape closes only the top dialog.</DialogDescription>
            </DialogHeader>
            <Dialog>
              <DialogTrigger asChild>
                <Button variant="danger">Archive project…</Button>
              </DialogTrigger>
              <DialogContent size="sm">
                <DialogHeader>
                  <DialogTitle>Are you sure?</DialogTitle>
                  <DialogDescription>This closes the project window.</DialogDescription>
                </DialogHeader>
              </DialogContent>
            </Dialog>
          </DialogContent>
        </Dialog>
        <Dialog>
          <DialogTrigger asChild>
            <Button>No close button</Button>
          </DialogTrigger>
          <DialogContent hideClose size="sm">
            <DialogHeader>
              <DialogTitle>Must choose</DialogTitle>
              <DialogDescription>Escape still closes it.</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>
      </section>
    </div>
  );
}
