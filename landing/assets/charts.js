/* CAVS site — tiny dependency-free SVG bar charts.
   Reads specs from <div class="chart" data-chart='{...}'> and renders
   grouped horizontal bars. Values are always printed, so short bars
   (huge dynamic range) stay honest and readable. */
(function () {
  "use strict";

  var W = 1000;          // viewBox width units
  var LABEL_W = 150;     // left gutter for series labels
  var PAD_R = 96;        // right room for value text
  var ROW_H = 30;        // bar row height
  var BAR_H = 15;        // bar thickness
  var GROUP_GAP = 20;    // gap between groups
  var HEAD_H = 26;       // group header height

  function esc(s) { return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;"); }

  function render(el) {
    var spec;
    try { spec = JSON.parse(el.getAttribute("data-chart")); } catch (e) { return; }
    var groups = spec.groups || [];
    var maxV = spec.max || 0;
    groups.forEach(function (g) {
      (g.bars || []).forEach(function (b) { if (b.v > maxV) maxV = b.v; });
    });
    if (maxV <= 0) maxV = 1;

    var barAreaW = W - LABEL_W - PAD_R;
    var y = 0;
    var rows = [];
    groups.forEach(function (g, gi) {
      if (gi > 0) y += GROUP_GAP;
      rows.push('<text class="bar-label" x="0" y="' + (y + 16) +
        '" style="font-weight:600;fill:var(--text)">' + esc(g.label) + "</text>");
      y += HEAD_H;
      (g.bars || []).forEach(function (b) {
        var cy = y + (ROW_H - BAR_H) / 2;
        var w = Math.max(2, (b.v / maxV) * barAreaW);
        var cls = b.cls || "bar-b";
        rows.push('<text class="bar-label" x="' + (LABEL_W - 12) + '" y="' + (cy + BAR_H - 3) +
          '" text-anchor="end">' + esc(b.name) + "</text>");
        rows.push('<rect class="bar-track" x="' + LABEL_W + '" y="' + cy +
          '" width="' + barAreaW + '" height="' + BAR_H + '" rx="4"></rect>');
        rows.push('<rect class="' + cls + '" x="' + LABEL_W + '" y="' + cy +
          '" width="' + w + '" height="' + BAR_H + '" rx="4"></rect>');
        var vx = LABEL_W + w + 10;
        var vcls = (cls === "bar-a") ? "bar-value dim" : "bar-value";
        rows.push('<text class="' + vcls + '" x="' + vx + '" y="' + (cy + BAR_H - 2) +
          '">' + esc(b.text) + "</text>");
        y += ROW_H;
      });
    });

    var H = y + 4;
    var svg = '<svg viewBox="0 0 ' + W + " " + H + '" role="img" ' +
      'preserveAspectRatio="xMinYMin meet" aria-label="' + esc(spec.title || "chart") + '">' +
      rows.join("") + "</svg>";

    var html = "";
    if (spec.title) html += '<h3>' + esc(spec.title) + "</h3>";
    if (spec.subtitle) html += '<div class="chart-sub">' + esc(spec.subtitle) + "</div>";
    html += svg;
    if (spec.legend) {
      var lg = spec.legend.map(function (l) {
        return '<span><i class="' + (l.cls || "b").replace("bar-", "") + '"></i>' + esc(l.name) + "</span>";
      }).join("");
      html += '<div class="chart-legend">' + lg + "</div>";
    }
    el.innerHTML = html;
  }

  function init() {
    document.querySelectorAll(".chart[data-chart]").forEach(render);
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();
