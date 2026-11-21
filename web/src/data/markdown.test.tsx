import { renderToStaticMarkup } from "react-dom/server";
import { expect, test } from "vitest";
import { Markdown } from "./markdown";

// Keep this fixture at the rendering boundary, including encoded schemes.
test("agent links cannot navigate the UI tab", () => {
  const html = renderToStaticMarkup(
    <Markdown>
      {
        "[web](https://example.com) [mail](mailto:a@example.com) [relative](/settings) [scheme](javascript:alert%281%29) [encoded](java&#x73;cript:alert%281%29)"
      }
    </Markdown>,
  );
  expect(html).toContain('target="_blank"');
  expect(html).toContain('rel="noopener noreferrer"');
  expect(html.match(/<a /g)).toHaveLength(1);
});

// A forge comment (the pull request pane, TRACKER 430) follows https only.
test("a forge comment links over https only", () => {
  const html = renderToStaticMarkup(
    <Markdown httpsOnly>
      {"[secure](https://example.com) [plain](http://example.com)"}
    </Markdown>,
  );
  expect(html.match(/<a /g)).toHaveLength(1);
  expect(html).toContain('href="https://example.com"');
});

// A forge comment's image is not loaded: its words link to it over https.
test("a forge comment's image becomes a link", () => {
  const html = renderToStaticMarkup(
    <Markdown httpsOnly>
      {"![trace](https://example.com/a.png) ![x](http://example.com/b.png)"}
    </Markdown>,
  );
  expect(html).not.toContain("<img");
  expect(html).toContain('<a href="https://example.com/a.png"');
  expect(html.match(/<a /g)).toHaveLength(1);
});
