/* app.js — {{project_name}} SolverForge UI */

(async function () {
  'use strict';

  var config = await fetch('/sf-config.json').then(function (r) { return r.json(); });

  var app = document.getElementById('sf-app');

  // Backend and solver
  var backend = SF.createBackend({ baseUrl: '' });
  var statusBar = SF.createStatusBar({ constraints: config.constraints });
  var solver = SF.createSolver({
    backend: backend,
    statusBar: statusBar,
    onUpdate: function (data) { renderSequences(data); renderTables(data); },
    onComplete: function (data) { renderSequences(data); renderTables(data); },
  });

  // Header
  var header = SF.createHeader({
    logo: '/sf/img/solverforge-horizontal.svg',
    title: config.title,
    subtitle: config.subtitle,
    tabs: [
      { id: 'sequences', label: 'Sequences', icon: 'fa-list-ol', active: true },
      { id: 'data', label: 'Data', icon: 'fa-table' },
      { id: 'api', label: 'REST API', icon: 'fa-book' },
    ],
    actions: {
      onSolve: function () { loadAndSolve(); },
      onStop: function () { solver.stop(); },
      onAnalyze: function () { openAnalysis(); },
    },
    onTabChange: function (tab) {
      sequencesPanel.style.display = tab === 'sequences' ? '' : 'none';
      dataPanel.style.display = tab === 'data' ? '' : 'none';
      apiPanel.style.display = tab === 'api' ? '' : 'none';
    },
  });
  app.appendChild(header);
  app.appendChild(statusBar.el);

  // Sequences panel (hero)
  var sequencesPanel = SF.el('div', { className: 'sf-content' });
  var sequencesContainer = SF.el('div', { id: 'sf-sequences' });
  sequencesPanel.appendChild(sequencesContainer);
  app.appendChild(sequencesPanel);

  // Data panel
  var dataPanel = SF.el('div', { className: 'sf-content', style: { display: 'none' } });
  var tablesContainer = SF.el('div', { id: 'sf-tables' });
  dataPanel.appendChild(tablesContainer);
  app.appendChild(dataPanel);

  // API panel
  var apiPanel = SF.el('div', { className: 'sf-content', style: { display: 'none' } });
  var guide = SF.createApiGuide({
    endpoints: [
      { method: 'GET', path: '/demo-data/STANDARD', description: 'Fetch demo data', curl: 'curl http://localhost:7860/demo-data/STANDARD' },
      { method: 'POST', path: '/schedules', description: 'Submit a plan for solving', curl: 'curl -X POST -H "Content-Type: application/json" http://localhost:7860/schedules -d @plan.json' },
      { method: 'GET', path: '/schedules/{id}', description: 'Get current best solution', curl: 'curl http://localhost:7860/schedules/{id}' },
      { method: 'GET', path: '/schedules/{id}/events', description: 'Stream solver updates (SSE)', curl: 'curl -N http://localhost:7860/schedules/{id}/events' },
      { method: 'GET', path: '/schedules/{id}/analyze', description: 'Get constraint analysis', curl: 'curl http://localhost:7860/schedules/{id}/analyze' },
      { method: 'DELETE', path: '/schedules/{id}', description: 'Stop solving and remove job', curl: 'curl -X DELETE http://localhost:7860/schedules/{id}' },
    ],
  });
  apiPanel.appendChild(guide);
  app.appendChild(apiPanel);

  // Footer
  var footer = SF.createFooter({
    links: [
      { label: 'SolverForge', url: 'https://www.solverforge.org' },
      { label: 'Docs', url: 'https://www.solverforge.org/docs' },
    ],
  });
  app.appendChild(footer);

  // Analysis modal
  var analysisModal = SF.createModal({ title: 'Score Analysis', width: '700px' });

  // Load demo data on startup
  fetch('/demo-data/STANDARD')
    .then(function (r) { return r.json(); })
    .then(function (data) { renderSequences(data); renderTables(data); })
    .catch(function () {});

  function loadAndSolve() {
    fetch('/demo-data/STANDARD')
      .then(function (r) { return r.json(); })
      .then(function (data) { solver.start(data); })
      .catch(function (err) { console.error('Demo load failed:', err); });
  }

  function openAnalysis() {
    var id = solver.getJobId();
    if (!id) return;
    backend.analyze(id)
      .then(function (analysis) {
        analysisModal.setBody(buildAnalysisHtml(analysis));
        analysisModal.open();
      })
      .catch(function () {});
  }

  function buildAnalysisHtml(analysis) {
    if (!analysis || !analysis.constraints) return '<p>No analysis available.</p>';
    var html = '<p><strong>Score:</strong> ' + SF.escHtml(analysis.score) + '</p>';
    html += '<table class="sf-table"><thead><tr><th>Constraint</th><th>Type</th><th>Score</th><th>Matches</th></tr></thead><tbody>';
    analysis.constraints.forEach(function (c) {
      html += '<tr><td>' + SF.escHtml(c.name) + '</td><td>' + SF.escHtml(c.type) + '</td><td>' + SF.escHtml(c.score) + '</td><td>' + (c.matches ? c.matches.length : 0) + '</td></tr>';
    });
    html += '</tbody></table>';
    return html;
  }

  function renderSequences(data) {
    sequencesContainer.innerHTML = '';
    var containers = data.containers || [];
    if (!containers.length) return;

    var cols = ['Container', 'Item Sequence', 'Count'];
    var rows = containers.map(function (c) {
      var seq = (c.items || []).join(' → ') || '—';
      return [c.name, seq, String((c.items || []).length)];
    });

    var section = SF.el('div', { className: 'sf-section' });
    section.appendChild(SF.createTable({ columns: cols, rows: rows }));
    sequencesContainer.appendChild(section);
  }

  function renderTables(data) {
    tablesContainer.innerHTML = '';

    config.entities.forEach(function (entity) {
      var items = data[entity.plural] || data[entity.name + 's'] || [];
      if (!items.length) return;
      var cols = Object.keys(items[0]);
      var rows = items.map(function (item) {
        return cols.map(function (k) {
          var v = item[k];
          if (v === null || v === undefined) return '—';
          if (Array.isArray(v)) return v.join(', ');
          if (typeof v === 'object') return JSON.stringify(v);
          return String(v);
        });
      });
      var section = SF.el('div', { className: 'sf-section' });
      section.appendChild(SF.el('h3', null, entity.label));
      section.appendChild(SF.createTable({ columns: cols, rows: rows }));
      tablesContainer.appendChild(section);
    });

    config.facts.forEach(function (fact) {
      var items = data[fact.plural] || data[fact.name + 's'] || [];
      if (!items.length) return;
      var cols = Object.keys(items[0]);
      var rows = items.map(function (item) {
        return cols.map(function (k) {
          var v = item[k];
          if (v === null || v === undefined) return '—';
          if (typeof v === 'object') return JSON.stringify(v);
          return String(v);
        });
      });
      var section = SF.el('div', { className: 'sf-section' });
      section.appendChild(SF.el('h3', null, fact.label));
      section.appendChild(SF.createTable({ columns: cols, rows: rows }));
      tablesContainer.appendChild(section);
    });
  }

})();
