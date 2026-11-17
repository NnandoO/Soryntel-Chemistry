import type { Metadata, Viewport } from "next";
import localFont from "next/font/local";
import { SITE_URL } from "@/content/site";
import "./globals.css";

// Self-hosted Geist (SIL OFL), variable weight; covers Spanish via Latin‑1.
const sans = localFont({ src: "./fonts/geist-latin.woff2", variable: "--font-sans", weight: "100 900", display: "swap" });
const mono = localFont({ src: "./fonts/geist-mono-latin.woff2", variable: "--font-geist-mono", weight: "100 900", display: "swap" });

const title = "Soryntel Chemistry — termodinámica aplicada para el aula y el laboratorio";
const description =
  "Suite de escritorio gratuita y de código abierto para equilibrio químico, agua y vapor IF97, reacciones, balances y procesamiento mineral. Linux y Windows.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title,
  description,
  alternates: { canonical: "/" },
  openGraph: {
    type: "website",
    locale: "es",
    url: "/",
    siteName: "Soryntel Chemistry",
    title,
    description,
    images: [{ url: "/media/water.jpg", width: 1440, height: 872, alt: "Calculadora de agua y vapor IF97 de Soryntel Chemistry" }],
  },
  twitter: { card: "summary_large_image", title, description, images: ["/media/water.jpg"] },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f4f6f3" },
    { media: "(prefers-color-scheme: dark)", color: "#121a18" },
  ],
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="es" className={`${sans.variable} ${mono.variable} antialiased`}>
      <body className="min-h-dvh">{children}</body>
    </html>
  );
}
