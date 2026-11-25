import Link from "next/link";
import { notFound } from "next/navigation";
import { DOCS, findDoc } from "@/lib/docs";

export function generateStaticParams() {
  return DOCS.map((page) => ({ slug: page.slug ? [page.slug] : [] }));
}

export default async function DocsPage({ params }: { params: Promise<{ slug?: string[] }> }) {
  const { slug } = await params;
  const key = slug?.[0] ?? "";
  if (slug && slug.length > 1) notFound();
  if (key && !DOCS.some((page) => page.slug === key)) notFound();
  const page = findDoc(key);
  return (
    <article className="docs">
      <nav>
        {DOCS.map((item) => (
          <Link key={item.slug || "home"} href={item.slug ? `/docs/${item.slug}` : "/docs"}>
            {item.title}
          </Link>
        ))}
      </nav>
      <h1>{page.title}</h1>
      {page.summary.map((line) => (
        <p className="summary" key={line}>
          {line}
        </p>
      ))}
      {page.sections.map((section) => (
        <section key={section.heading}>
          <h2>{section.heading}</h2>
          {section.body.map((line) => (
            <p key={line}>{line}</p>
          ))}
        </section>
      ))}
      <p>
        <Link href="/">Back to the desk</Link>
      </p>
    </article>
  );
}
