import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  serverExternalPackages: ["ssh2", "cpu-features", "better-sqlite3"],
};

export default nextConfig;
