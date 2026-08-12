import preact from "@preact/preset-vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [preact()],
  build: {
    target: "es2022",
  },
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  test: {
    environment: "node",
  },
});
