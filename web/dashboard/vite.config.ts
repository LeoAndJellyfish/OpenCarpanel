import preact from "@preact/preset-vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    preact(),
    {
      name: "opencarpanel-development-csp",
      apply: "serve",
      transformIndexHtml(html) {
        return html.replace("style-src 'self';", "style-src 'self' 'unsafe-inline';");
      },
    },
  ],
  build: {
    target: "es2022",
  },
  server: {
    host: true,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:20778",
        ws: true,
      },
    },
  },
  test: {
    environment: "node",
  },
});
