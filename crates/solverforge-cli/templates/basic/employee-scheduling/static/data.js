// Demo data fetching, dataset switching, and schedule refresh

function fetchDemoData() {
    $.get("/demo-data", function (data) {
        data.forEach(item => {
            $("#testDataButton").append($('<a id="' + item + 'TestData" class="dropdown-item" href="#">' + item + '</a>'));
            $("#" + item + "TestData").click(function () {
                switchDataDropDownItemActive(item);
                scheduleId = null;
                demoDataId = item;

                refreshSchedule();
            });
        });
        demoDataId = data[0];
        switchDataDropDownItemActive(demoDataId);
        refreshSchedule();
    }).fail(function (xhr, ajaxOptions, thrownError) {
        // disable this page as there is no data
        let $demo = $("#demo");
        $demo.empty();
        $demo.html("<h1><p align=\"center\">No test data available</p></h1>")
    });
}

function switchDataDropDownItemActive(newItem) {
    activeCssClass = "active";
    $("#testDataButton > a." + activeCssClass).removeClass(activeCssClass);
    $("#" + newItem + "TestData").addClass(activeCssClass);
}

function refreshSchedule() {
    let path = "/schedules/" + scheduleId;
    if (scheduleId === null) {
        if (demoDataId === null) {
            alert("Please select a test data set.");
            return;
        }

        path = "/demo-data/" + demoDataId;
    }
    $.getJSON(path, function (schedule) {
        loadedSchedule = schedule;
        renderSchedule(schedule);
    })
        .fail(function (xhr, ajaxOptions, thrownError) {
            showError("Getting the schedule has failed.", xhr);
            refreshSolvingButtons(false);
        });
}
