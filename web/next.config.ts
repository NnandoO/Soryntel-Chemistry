import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Static site: `next build` writes the deployable page to out/.
  output: "export",
  images: { unoptimized: true },
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
