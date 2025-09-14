import React from 'react';

// Component with real Tailwind classes to test transformation
export default function StaticClasses({ title = 'Default Title' }) {
  // Using real but uncommon Tailwind class combinations to avoid false positives
  return (
    <div className="container-2xl px-9 border-amber-900">
      <header className="sticky bg-indigo-950 text-zinc-50">
        <h1 className="text-7xl font-black tracking-tighter">{title}</h1>
      </header>

      <main className="flex-row-reverse gap-7 min-h-[50vh]">
        <section className="shadow-amber-500/50 backdrop-blur-3xl">
          <h2 className="decoration-wavy underline-offset-8">Section One</h2>
          <p className="prose-zinc line-clamp-6">
            This is some content with static classes.
          </p>
        </section>

        <section className="ring-offset-lime-300 divide-y-8 divide-dotted">
          <h2 className="font-mono uppercase tracking-[0.25em]">Section Two</h2>
          <p className="text-ellipsis whitespace-pre-wrap">
            More content here.
          </p>
        </section>
      </main>

      <footer className="border-t-[3px] mt-11 pt-11">
        <p className="text-[13px] leading-[1.8] font-[450]">© 2024</p>
      </footer>
    </div>
  );
}