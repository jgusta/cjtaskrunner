function cjTasksLanguage(hljs) {
  const variableReference =
    /(?<!\\)(?:\$\{[A-Za-z_][A-Za-z0-9_]*(?:\?[^}]*)?\}|\$[A-Za-z_][A-Za-z0-9_]*)/;
  const captureTarget =
    /(?:\$\{[A-Za-z_][A-Za-z0-9_]*(?:\?[^}]*)?\}|\$[A-Za-z_][A-Za-z0-9_]*|[A-Za-z_][A-Za-z0-9_]*)/;
  const variable = {
    className: "variable",
    begin: variableReference,
    relevance: 0
  };
  const escape = { begin: /\\./, relevance: 0 };
  const strings = [
    {
      className: "string",
      begin: /"/,
      end: /"|$/,
      contains: [escape, variable]
    },
    {
      className: "string",
      begin: /'/,
      end: /'|$/,
      contains: [escape]
    }
  ];
  const taskArguments = {
    begin: /\(/,
    end: /\)/,
    contains: [
      { className: "variable", begin: /[A-Za-z_][A-Za-z0-9_]*/, relevance: 0 },
      { className: "symbol", begin: /,/, relevance: 0 }
    ]
  };

  return {
    name: "CJTaskrunner",
    aliases: ["cjtasks"],
    contains: [
      {
        className: "comment",
        begin: /^\s*#/m,
        end: /$/,
        relevance: 0
      },
      {
        begin: /^\s*(?=[A-Za-z_][A-Za-z0-9_]*(?:\?:\s*.*|:\s+\S.*)$)/m,
        end: /$/,
        contains: [
          { className: "variable", begin: /[A-Za-z_][A-Za-z0-9_]*/, relevance: 0 },
          { className: "symbol", begin: /\??:/, relevance: 0 },
          ...strings,
          variable
        ]
      },
      {
        begin: /^\s*(?=[A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)*(?:\s+\([A-Za-z_][A-Za-z0-9_]*(?:\s*,\s*[A-Za-z_][A-Za-z0-9_]*)*\))?:\s*$)/m,
        end: /$/,
        contains: [
          taskArguments,
          { className: "title", begin: /[A-Za-z0-9_-]+/, relevance: 0 },
          { className: "symbol", begin: /:/, relevance: 0 }
        ]
      },
      {
        begin: new RegExp(
          "^\\s*(?=@(?:help|env):\\s*$|@set\\s+" + captureTarget.source + ":\\s*$)",
          "m"
        ),
        end: /$/,
        contains: [
          { className: "meta", begin: /@(?:help|env|set)\b/, relevance: 0 },
          {
            className: "variable",
            begin: new RegExp(captureTarget.source + "(?=\\s*:)"),
            relevance: 0
          },
          { className: "symbol", begin: /:/, relevance: 0 }
        ]
      },
      {
        className: "comment",
        begin: /^\s*(?=@desc(?:\s|$))/m,
        end: /$/,
        contains: [
          { className: "keyword", begin: /@desc\b/, relevance: 0 },
          variable
        ]
      },
      {
        begin: /@task\s+/,
        end: /$/,
        returnBegin: true,
        contains: [
          { className: "keyword", begin: /@task\b/, relevance: 0 },
          {
            className: "title",
            begin: /[A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)*/,
            endsParent: true,
            relevance: 0
          }
        ]
      },
      ...strings,
      variable,
      {
        className: "keyword",
        begin: /@[A-Za-z][A-Za-z0-9-]*/,
        relevance: 0
      }
    ]
  };
}

function highlightCjTasksBlocks() {
  if (typeof hljs === "undefined" || typeof document === "undefined") {
    return;
  }
  if (!hljs.getLanguage("cjtasks")) {
    hljs.registerLanguage("cjtasks", cjTasksLanguage);
  }
  document.querySelectorAll("code.language-cjtasks").forEach((block) => {
    hljs.highlightBlock(block);
  });
}

highlightCjTasksBlocks();

if (typeof document !== "undefined") {
  document.addEventListener("DOMContentLoaded", () => {
    const main = document.querySelector("main");
    if (!main) {
      return;
    }

    const button = document.createElement("button");
    button.type = "button";
    button.className = "cj-back-button";
    button.setAttribute("aria-label", "Go back");
    button.title = "Go back";
    button.textContent = "\u2190";
    button.addEventListener("click", () => window.history.back());
    main.insertBefore(button, main.firstChild);
  });
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = cjTasksLanguage;
}
