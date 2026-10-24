// One example page per primitive, loaded lazily. registry.test.tsx fails until every key of PRIMITIVES has a page.
import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import type { PrimitiveName } from "../../components/ui";

export type ExamplePage = { name: PrimitiveName; blurb: string; Page: LazyExoticComponent<ComponentType> };

function page(name: PrimitiveName, blurb: string, load: () => Promise<{ default: ComponentType }>): ExamplePage {
  return { name, blurb, Page: lazy(load) };
}

export const pages: ExamplePage[] = [
  page("Button", "Primary, secondary, ghost, danger and link buttons", () => import("./pages/ButtonPage")),
  page("IconButton", "Square icon buttons with a required label", () => import("./pages/IconButtonPage")),
];
