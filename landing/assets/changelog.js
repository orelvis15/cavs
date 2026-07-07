/* CAVS site — changelog fetched live from the repository's CHANGELOG.md
   and rendered with a small Keep-a-Changelog-aware markdown renderer.
   No dependencies. */
(function () {
  "use strict";

  var REPO = "orelvis15/cavs";
  var BRANCH = "main";
  var RAW = "https://raw.githubusercontent.com/" + REPO + "/" + BRANCH + "/CHANGELOG.md";
  var BLOB = "https://github.com/" + REPO + "/blob/" + BRANCH + "/";

  var out = document.getElementById("changelog");
  if (!out) return;

  function esc(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  // links, bold and italic — applied only to non-code segments
  function fmt(s) {
    return s
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, function (_, text, href) {
        var url = /^https?:\/\//.test(href) ? href : BLOB + href.replace(/^\.\//, "");
        return '<a href="' + url + '" target="_blank" rel="noopener">' + text + "</a>";
      })
      .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
      .replace(/\*([^*\n]+)\*/g, "<em>$1</em>");
  }

  // Split on backtick-delimited code spans so their contents are never
  // touched by the link/bold/italic passes. Odd segments are code.
  function inline(s) {
    var parts = esc(s).split("`");
    var res = "";
    for (var k = 0; k < parts.length; k++) {
      res += (k % 2 === 1) ? "<code>" + parts[k] + "</code>" : fmt(parts[k]);
    }
    return res;
  }

  function indentOf(line) { var m = line.match(/^(\s*)/); return m ? m[1].length : 0; }

  // Render a release body (array of lines) into HTML blocks.
  function renderBody(lines) {
    var html = "";
    var i = 0;
    while (i < lines.length) {
      var line = lines[i];
      if (/^\s*$/.test(line)) { i++; continue; }

      if (/^###\s+/.test(line)) {
        html += "<h3>" + inline(line.replace(/^###\s+/, "")) + "</h3>";
        i++;
        continue;
      }

      if (/^\s*[-*]\s+/.test(line)) {
        var res = renderList(lines, i, indentOf(line));
        html += res.html;
        i = res.next;
        continue;
      }

      // paragraph: gather consecutive plain lines
      var para = [];
      while (i < lines.length && !/^\s*$/.test(lines[i]) &&
             !/^###\s+/.test(lines[i]) && !/^\s*[-*]\s+/.test(lines[i])) {
        para.push(lines[i].trim());
        i++;
      }
      html += "<p>" + inline(para.join(" ")) + "</p>";
    }
    return html;
  }

  // Render a (possibly nested) bullet list starting at index `start`.
  function renderList(lines, start, baseIndent) {
    var html = "<ul>";
    var i = start;
    while (i < lines.length) {
      var line = lines[i];
      if (/^\s*$/.test(line)) { i++; continue; }
      if (!/^\s*[-*]\s+/.test(line)) break;
      var ind = indentOf(line);
      if (ind < baseIndent) break;
      var text = line.replace(/^\s*[-*]\s+/, "");
      // continuation lines (wrapped) belong to this item
      var cont = [text];
      i++;
      while (i < lines.length && !/^\s*$/.test(lines[i]) &&
             !/^\s*[-*]\s+/.test(lines[i]) && !/^###\s+/.test(lines[i])) {
        cont.push(lines[i].trim());
        i++;
      }
      var itemHtml = "<li>" + inline(cont.join(" "));
      // nested list?
      if (i < lines.length && /^\s*[-*]\s+/.test(lines[i]) && indentOf(lines[i]) > baseIndent) {
        var nested = renderList(lines, i, indentOf(lines[i]));
        itemHtml += nested.html;
        i = nested.next;
      }
      itemHtml += "</li>";
      html += itemHtml;
    }
    html += "</ul>";
    return { html: html, next: i };
  }

  function badgeFor(ver) {
    if (/unreleased/i.test(ver)) return '<span class="badge soon">unreleased</span>';
    return "";
  }

  function render(md) {
    var lines = md.split(/\r?\n/);
    // find release boundaries (## ...)
    var releases = [];
    var current = null;
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i];
      if (/^##\s+/.test(line)) {
        if (current) releases.push(current);
        current = { header: line.replace(/^##\s+/, "").trim(), body: [] };
      } else if (current) {
        current.body.push(line);
      }
    }
    if (current) releases.push(current);

    if (!releases.length) {
      out.innerHTML = '<p class="cl-status error">Could not parse the changelog.</p>';
      return;
    }

    var html = "";
    releases.forEach(function (r) {
      var bodyText = r.body.join("").trim();
      // skip an empty "Unreleased" placeholder so the page opens on a real release
      if (!bodyText && /unreleased/i.test(r.header)) return;
      // header like "[0.8.0] — Auto-route optimized delivery"
      var m = r.header.match(/^\[([^\]]+)\]\s*[—-]?\s*(.*)$/);
      var ver = m ? m[1] : r.header;
      var title = m ? m[2] : "";
      html += '<div class="cl-release">';
      html += '<div class="cl-ver"><h2>' + esc(ver) + "</h2>";
      if (title) html += '<span class="cl-title">' + inline(title) + "</span>";
      html += badgeFor(r.header);
      html += "</div>";
      var body = renderBody(r.body).trim();
      html += '<div class="cl-body">' + (body || "<p>No notes.</p>") + "</div>";
      html += "</div>";
    });
    out.innerHTML = html;
  }

  out.innerHTML = '<p class="cl-status">Loading changelog from the repository…</p>';
  fetch(RAW, { cache: "no-cache" })
    .then(function (r) {
      if (!r.ok) throw new Error("HTTP " + r.status);
      return r.text();
    })
    .then(render)
    .catch(function () {
      out.innerHTML =
        '<p class="cl-status error">Could not load the live changelog.<br>' +
        'Read it directly on <a href="https://github.com/' + REPO + '/blob/' + BRANCH +
        '/CHANGELOG.md" target="_blank" rel="noopener" style="color:var(--blue)">GitHub →</a></p>';
    });
})();
