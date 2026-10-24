import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Button } from "@/components/ui";
import { useConfirm } from "./ConfirmDialog";

function Host({ onAnswer }: { onAnswer(ok: boolean): void }) {
  const { confirm, element } = useConfirm();
  return (
    <>
      <Button
        onClick={() => void confirm({ title: "Archive it?", body: "Sure?", confirmLabel: "Archive" }).then(onAnswer)}
      >
        ask
      </Button>
      {element}
    </>
  );
}

describe("useConfirm", () => {
  it("resolves true on the confirm button and false on Cancel", async () => {
    const answers: boolean[] = [];
    render(<Host onAnswer={(ok) => answers.push(ok)} />);
    await userEvent.click(screen.getByText("ask"));
    expect(await screen.findByText("Archive it?")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Archive" }));
    await act(async () => {});
    expect(answers).toEqual([true]);
    expect(screen.queryByText("Archive it?")).toBeNull();
    await userEvent.click(screen.getByText("ask"));
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    await act(async () => {});
    expect(answers).toEqual([true, false]);
  });
  it("Escape counts as Cancel", async () => {
    const answers: boolean[] = [];
    render(<Host onAnswer={(ok) => answers.push(ok)} />);
    await userEvent.click(screen.getByText("ask"));
    await screen.findByText("Archive it?");
    await userEvent.keyboard("{Escape}");
    await act(async () => {});
    expect(answers).toEqual([false]);
  });
});
