// vis.Timeline setup for the by-employee and by-location panels

const byEmployeePanel = document.getElementById("byEmployeePanel");
const byEmployeeTimelineOptions = {
    timeAxis: {scale: "hour", step: 6},
    orientation: {axis: "top"},
    stack: false,
    xss: {disabled: true}, // Items are XSS safe through JQuery
    zoomMin: zoomMin,
    zoomMax: zoomMax,
};
let byEmployeeGroupDataSet = new vis.DataSet();
let byEmployeeItemDataSet = new vis.DataSet();
let byEmployeeTimeline = new vis.Timeline(byEmployeePanel, byEmployeeItemDataSet, byEmployeeGroupDataSet, byEmployeeTimelineOptions);

const byLocationPanel = document.getElementById("byLocationPanel");
const byLocationTimelineOptions = {
    timeAxis: {scale: "hour", step: 6},
    orientation: {axis: "top"},
    xss: {disabled: true}, // Items are XSS safe through JQuery
    zoomMin: zoomMin,
    zoomMax: zoomMax,
};
let byLocationGroupDataSet = new vis.DataSet();
let byLocationItemDataSet = new vis.DataSet();
let byLocationTimeline = new vis.Timeline(byLocationPanel, byLocationItemDataSet, byLocationGroupDataSet, byLocationTimelineOptions);

let windowStart = JSJoda.LocalDate.now().toString();
let windowEnd = JSJoda.LocalDate.parse(windowStart).plusDays(7).toString();
