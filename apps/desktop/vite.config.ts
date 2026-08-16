import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;
const rootDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(rootDir, "../..");

export default defineConfig(async () => ({
  plugins: [react()],
  resolve: {
    alias: {
      "@lunatic-asylum/shared": path.join(repoRoot, "packages/shared/src/index.ts"),
      "@lunatic-asylum/core": path.join(repoRoot, "packages/core/src/index.ts"),
      "@lunatic-asylum/provider-palworld": path.join(
        repoRoot,
        "packages/providers/palworld/src/index.ts",
      ),
      "@lunatic-asylum/palworld-discord": path.join(
        repoRoot,
        "packages/integrations/palworld-discord/src/index.ts",
      ),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
