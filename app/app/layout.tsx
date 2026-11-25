import type { Metadata, Viewport } from "next";
import type { ReactNode } from "react";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/500.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "./globals.css";
import { Providers } from "@/components/Providers";

export const metadata: Metadata = {
  title: "Mark",
  description: "A perpetual on event odds. Trade the probability, not the asset.",
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: "#0c0914",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <Providers>
          <div className="shell">
            <main>{children}</main>
            <footer className="foot">
              <span>Built and developed by <a href="https://saddammalik.com">Saddam Malik</a></span>
              <span>Unaudited. Devnet demo. Not an offer to run an exchange.</span>
            </footer>
          </div>
        </Providers>
      </body>
    </html>
  );
}
