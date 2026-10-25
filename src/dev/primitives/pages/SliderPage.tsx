import { useState, type ReactElement } from "react";
import { Field, Slider } from "../../../components/ui";

export default function SliderPage(): ReactElement {
  const [replay, setReplay] = useState(9);
  const [zoom, setZoom] = useState(100);
  return (
    <div className="flex max-w-md flex-col gap-4">
      <h1 className="text-lg font-semibold">Slider</h1>
      <Field label="Replay position" hint={`${replay} of 18 events`}>
        <Slider value={replay} min={1} max={18} onChange={setReplay} label="Replay position" />
      </Field>
      <Field label="Zoom" hint="Steps of 25, with value text">
        <Slider
          value={zoom}
          min={25}
          max={200}
          step={25}
          onChange={setZoom}
          label="Zoom"
          formatValue={(v) => `${v} percent`}
        />
      </Field>
      <Slider value={50} min={0} max={100} onChange={() => {}} label="Disabled" disabled />
    </div>
  );
}
