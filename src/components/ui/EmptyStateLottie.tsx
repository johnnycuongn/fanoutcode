// The dotLottie renderer, in its own chunk: EmptyState loads it with React.lazy only when it has an animation.
import { DotLottieReact, setWasmUrl } from "@lottiefiles/dotlottie-react";
import wasmUrl from "@lottiefiles/dotlottie-web/dotlottie-player.wasm?url";
import type { ReactElement } from "react";

// The renderer is WebAssembly. Serve it from the bundle (Vite `?url`) so nothing is fetched from a CDN (CSP 'self').
setWasmUrl(wasmUrl);

export default function EmptyStateLottie({
  src,
  size,
  reduced,
}: {
  src: string;
  size: number;
  reduced: boolean;
}): ReactElement {
  return <DotLottieReact src={src} loop={!reduced} autoplay={!reduced} style={{ width: size, height: size }} />;
}
