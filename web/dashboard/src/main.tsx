import { render } from "preact";

import { App } from "./app";
import "./styles/base.css";
import "./styles/dashboard.css";
import "./styles/grid.css";
import "./styles/motion.css";

const root = document.querySelector<HTMLDivElement>("#app");
if (!root) {
  throw new Error("OpenCarpanel app root is missing");
}

render(<App />, root);
