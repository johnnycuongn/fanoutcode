// The dotLottie renderer for the done moment, in its own chunk (DoneMoment loads it with React.lazy).
import { DotLottieReact, setWasmUrl } from "@lottiefiles/dotlottie-react";
import wasmUrl from "@lottiefiles/dotlottie-web/dotlottie-player.wasm?url";
import type { ReactElement } from "react";

// The renderer is WebAssembly. Serve it from the bundle so nothing is fetched from a CDN (CSP 'self').
setWasmUrl(wasmUrl);

export default function DoneLottie({
  src,
  size,
  onComplete,
}: {
  src: string;
  size: number;
  onComplete(): void;
}): ReactElement {
  return (
    <DotLottieReact
      src={src}
      loop={false}
      autoplay
      style={{ width: size, height: size }}
      dotLottieRefCallback={(player) => player?.addEventListener("complete", onComplete)}
    />
  );
}
