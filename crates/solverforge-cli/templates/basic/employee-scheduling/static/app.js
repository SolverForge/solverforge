// Application entry point: wires up event handlers and kicks off data loading.
// All logic lives in the companion modules loaded before this file.

$(document).ready(function () {
    let initialized = false;

    function safeInitialize() {
        if (!initialized) {
            initialized = true;
            initializeApp();
        }
    }

    // Ensure all resources are loaded before initializing
    $(window).on('load', safeInitialize);

    // Fallback if window load event doesn't fire
    setTimeout(safeInitialize, 100);
});

function initializeApp() {
    replaceQuickstartSolverForgeAutoHeaderFooter();

    $("#solveButton").click(function () {
        solve();
    });
    $("#stopSolvingButton").click(function () {
        stopSolving();
    });
    $("#analyzeButton").click(function () {
        analyze();
    });
    // HACK to allow vis-timeline to work within Bootstrap tabs
    $("#byEmployeeTab").on('shown.bs.tab', function (event) {
        byEmployeeTimeline.redraw();
    })
    $("#byLocationTab").on('shown.bs.tab', function (event) {
        byLocationTimeline.redraw();
    })

    setupAjax();
    fetchDemoData();
}
