import { createContext, useContext, useId, type ComponentProps, type ReactElement, type ReactNode } from "react";
import { cx } from "./cx";

export type FieldControlAttrs = {
  id: string;
  "aria-describedby"?: string;
  "aria-invalid"?: true;
  "aria-required"?: true;
};

const FieldContext = createContext<FieldControlAttrs | undefined>(undefined);

/** The ids and ARIA wiring of the surrounding Field; undefined outside one. Controls spread it under their own props. */
export function useFieldControl(): FieldControlAttrs | undefined {
  return useContext(FieldContext);
}

export type FieldProps = ComponentProps<"div"> & {
  label?: ReactNode;
  hint?: ReactNode;
  error?: ReactNode;
  required?: boolean;
  /** "row" puts the label left and the control right once the nearest `@container` ancestor is wide enough; narrower, or with no container, it stacks. */
  layout?: "stack" | "row";
  /** Base id; default `useId()`. */
  id?: string;
};

export function Field({
  label,
  hint,
  error,
  required,
  layout = "stack",
  id,
  className,
  children,
  ...props
}: FieldProps): ReactElement {
  const generated = useId();
  const base = id ?? generated;
  const hintId = hint ? `${base}-hint` : undefined;
  const errorId = error ? `${base}-error` : undefined;
  const describedBy = [hintId, errorId].filter(Boolean).join(" ") || undefined;
  const attrs: FieldControlAttrs = {
    id: base,
    ...(describedBy ? { "aria-describedby": describedBy } : {}),
    ...(error ? { "aria-invalid": true as const } : {}),
    ...(required ? { "aria-required": true as const } : {}),
  };
  const labelNode = label ? (
    <label htmlFor={base} className="text-sm font-medium text-ink">
      {label}
      {required && (
        <span aria-hidden="true" className="ml-0.5 text-bad">
          *
        </span>
      )}
    </label>
  ) : null;
  const rest = (
    <>
      {children}
      {hint && (
        <p id={hintId} className="text-xs text-mute">
          {hint}
        </p>
      )}
      {error && (
        <p id={errorId} role="alert" className="text-xs text-bad">
          {error}
        </p>
      )}
    </>
  );
  return (
    <FieldContext.Provider value={attrs}>
      <div
        className={cx(
          layout === "row"
            ? "grid grid-cols-1 items-start gap-y-1 @md:grid-cols-[minmax(8rem,1fr)_2fr] @md:gap-x-4"
            : "flex flex-col gap-1",
          className,
        )}
        {...props}
      >
        {labelNode}
        {layout === "row" ? <div className="flex min-w-0 flex-col gap-1">{rest}</div> : rest}
      </div>
    </FieldContext.Provider>
  );
}
