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
    onUpdate: function (data) { renderHero(data); renderTables(data); },
    onComplete: function (data) { renderHero(data); renderTables(data); },
  });

  // Header
  var header = SF.createHeader({
    logo: '/sf/img/solverforge-horizontal.svg',
    title: config.title,
    subtitle: config.subtitle,
    tabs: [
      { id: 'hero', label: heroLabel(), icon: heroIcon(), active: true },
      { id: 'data', label: 'Data', icon: 'fa-table' },
      { id: 'api', label: 'REST API', icon: 'fa-book' },
    ],
    actions: {
      onSolve: function () { loadAndSolve(); },
      onStop: function () { solver.stop(); },
      onAnalyze: function () { openAnalysis(); },
    },
    onTabChange: function (tab) {
      heroPanel.style.display = tab === 'hero' ? '' : 'none';
      dataPanel.style.display = tab === 'data' ? '' : 'none';
      apiPanel.style.display = tab === 'api' ? '' : 'none';
    },
  });
  app.appendChild(header);
  app.appendChild(statusBar.el);

  // Hero panel
  var heroPanel = SF.el('div', { className: 'sf-content' });
  var heroContainer = SF.el('div', { id: 'sf-hero' });
  heroPanel.appendChild(heroContainer);
  app.appendChild(heroPanel);

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
    .then(function (data) { renderHero(data); renderTables(data); })
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

  function heroLabel() {
    return isTimetableView() ? 'Timetable' : 'Assignments';
  }

  function heroIcon() {
    return isTimetableView() ? 'fa-calendar-days' : 'fa-table-cells-large';
  }

  function isTimetableView() {
    return config.view && config.view.type === 'timetable';
  }

  function renderHero(data) {
    if (isTimetableView()) {
      renderTimetable(data);
    } else {
      renderAssignmentBoard(data);
    }
  }

  function renderTimetable(data) {
    heroContainer.innerHTML = '';
    var resources = data.resources || [];
    var tasks = data.tasks || [];
    if (!resources.length) return;

    var fields = config.view && config.view.fields ? config.view.fields : {};
    var startField = fields.start;
    var endField = fields.end;
    var labelField = fields.label || 'name';
    var positionedTasks = tasks.filter(function (task) {
      return typeof task[startField] === 'number' && typeof task[endField] === 'number';
    });
    var maxEnd = positionedTasks.reduce(function (maxValue, task) {
      return Math.max(maxValue, task[endField]);
    }, 0);
    var numSlots = Math.max(maxEnd, 1);

    var hdr = SF.rail.createHeader({
      label: config.facts[0] ? config.facts[0].label : 'Resource',
      labelWidth: 160,
      columns: Array.from({ length: numSlots }, function (_, i) { return 'Slot ' + (i + 1); }),
    });
    heroContainer.appendChild(hdr);

    resources.forEach(function (res) {
      var assigned = tasks.filter(function (t) {
        return t.resource && t.resource.name === res.name;
      });
      var card = SF.rail.createCard({
        id: 'res-' + res.index,
        name: res.name,
        labelWidth: 160,
        columns: numSlots,
        stats: [{ label: 'Tasks', value: assigned.length }],
      });
      assigned.forEach(function (task) {
        if (typeof task[startField] !== 'number' || typeof task[endField] !== 'number') return;
        card.addBlock({
          label: String(task[labelField] || task.name || task.id || 'Item'),
          start: task[startField],
          end: task[endField],
          horizon: numSlots,
          color: SF.colors.pick(String(task[labelField] || task.name || task.id || 'Item')),
        });
      });
      heroContainer.appendChild(card.el);
    });
  }

  function renderAssignmentBoard(data) {
    heroContainer.innerHTML = '';
    var resources = data.resources || [];
    var tasks = data.tasks || [];
    var assignedByResource = {};
    var totalDemand = 0;
    var assignedDemand = 0;
    var affinityMatches = 0;

    resources.forEach(function (res) {
      assignedByResource[res.name] = [];
    });

    tasks.forEach(function (task) {
      totalDemand += Number(task.demand || 0);
      var resourceName = task.resource && task.resource.name;
      if (resourceName && assignedByResource[resourceName]) {
        assignedByResource[resourceName].push(task);
        assignedDemand += Number(task.demand || 0);
        if (task.resource.affinityGroup === task.preferredGroup) affinityMatches += 1;
      }
    });

    var totalCapacity = resources.reduce(function (sum, resource) {
      return sum + Number(resource.capacity || 0);
    }, 0);

    var summary = SF.el('div', { className: 'sf-section' });
    summary.appendChild(SF.el('h3', null, 'Assignment Overview'));
    summary.appendChild(SF.createTable({
      columns: ['Metric', 'Value'],
      rows: [
        ['Resources', String(resources.length)],
        ['Tasks', String(tasks.length)],
        ['Total capacity', String(totalCapacity)],
        ['Total demand', String(totalDemand)],
        ['Assigned', String(tasks.filter(function (task) { return !!task.resource; }).length)],
        ['Assigned demand', String(assignedDemand)],
        ['Affinity matches', String(affinityMatches)],
        ['Unassigned', String(tasks.filter(function (task) { return !task.resource; }).length)],
      ],
    }));
    heroContainer.appendChild(summary);

    resources
      .slice()
      .sort(function (a, b) {
        return resourceLoad(assignedByResource[b.name]) - resourceLoad(assignedByResource[a.name]);
      })
      .forEach(function (res) {
      heroContainer.appendChild(buildAssignmentSection(
        res,
        assignedByResource[res.name],
        'Tasks'
      ));
    });

    var unassigned = tasks.filter(function (task) { return !task.resource; });
    if (unassigned.length) {
      heroContainer.appendChild(buildAssignmentSection({
        name: 'Unassigned',
        capacity: 0,
        affinityGroup: '—',
      }, unassigned, 'Tasks'));
    }
  }

  function resourceLoad(tasks) {
    return tasks.reduce(function (sum, task) {
      return sum + Number(task.demand || 0);
    }, 0);
  }

  function buildAssignmentSection(resource, tasks, statLabel) {
    var section = SF.el('div', { className: 'sf-section' });
    var load = resourceLoad(tasks);
    var title = resource.name;
    if (resource.capacity) {
      title += ' (' + load + '/' + resource.capacity + ' load)';
    } else {
      title += ' (' + tasks.length + ')';
    }
    section.appendChild(SF.el('h3', null, title));
    if (!tasks.length) {
      section.appendChild(SF.el('p', null, 'No assigned entities.'));
      return section;
    }
    var matches = tasks.filter(function (task) {
      return task.preferredGroup === resource.affinityGroup;
    }).length;
    section.appendChild(SF.createTable({
      columns: ['Affinity group', 'Capacity', 'Load', 'Preference matches'],
      rows: [[
        resource.affinityGroup || '—',
        String(resource.capacity || 0),
        String(load),
        String(matches),
      ]],
    }));
    section.appendChild(SF.createTable({
      columns: ['Entity', 'Id', 'Demand', 'Preferred group', statLabel],
      rows: tasks
        .slice()
        .sort(function (a, b) { return Number(b.demand || 0) - Number(a.demand || 0); })
        .map(function (task, index) {
        return [
          task.name || 'Unnamed',
          task.id || '—',
          String(task.demand || 0),
          task.preferredGroup || '—',
          String(index + 1),
        ];
      }),
    }));
    return section;
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
