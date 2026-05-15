export function renderPage() {
  const greeting = process.env.SSR_GREETING || "Hello";
  return `<!doctype html>
<html lang="en">
  <head><meta charset="utf-8"><title>CJTaskrunner SSR Example</title></head>
  <body><h1>${greeting}</h1><p>Rendered on the server.</p></body>
</html>`;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  console.log(renderPage());
}
