import type { Metadata } from "next";
import type { ReactNode } from "react";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/500.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "./globals.css";
import { Providers } from "@/components/Providers";

export const metadata: Metadata = {
  title: "Mark",
  description: "Perpetual markets on event odds. Trade the probability, not the asset.",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <Providers>
          <div className="shell">
            <main>{children}</main>
            <footer className="foot">
              Built and developed by <a href="https://saddammalik.com">Saddam Malik</a>
            </footer>
          </div>
        </Providers>
      </body>
    </html>
  );
}
