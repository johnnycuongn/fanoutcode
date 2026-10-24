import type { ReactElement } from "react";
import { UiProvider } from "../../components/ui";
import { Gallery } from "./Gallery";

export default function PrimitivesGallery(): ReactElement {
  return (
    <UiProvider>
      <Gallery />
    </UiProvider>
  );
}
