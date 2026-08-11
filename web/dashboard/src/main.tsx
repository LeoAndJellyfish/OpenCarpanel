import { render } from "preact";

import { App } from "./app";
import "./styles/base.css";

const root = document.querySelector<HTMLDivElement>("#app");
if (!root) {
  throw new Error("OpenCarpanel app root is missing");
}

render(<App />, root);
