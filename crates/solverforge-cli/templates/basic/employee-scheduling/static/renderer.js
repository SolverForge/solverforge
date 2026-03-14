// Schedule rendering: timeline population, shift coloring, unassigned shift display

function getShiftColor(shift, employee) {
    const shiftStart = JSJoda.LocalDateTime.parse(shift.start);
    const shiftStartDateString = shiftStart.toLocalDate().toString();
    const shiftEnd = JSJoda.LocalDateTime.parse(shift.end);
    const shiftEndDateString = shiftEnd.toLocalDate().toString();
    if (employee.unavailableDates.includes(shiftStartDateString) ||
        // The contains() check is ignored for a shift end at midnight (00:00:00).
        (shiftEnd.isAfter(shiftStart.toLocalDate().plusDays(1).atStartOfDay()) &&
            employee.unavailableDates.includes(shiftEndDateString))) {
        return UNAVAILABLE_COLOR
    } else if (employee.undesiredDates.includes(shiftStartDateString) ||
        // The contains() check is ignored for a shift end at midnight (00:00:00).
        (shiftEnd.isAfter(shiftStart.toLocalDate().plusDays(1).atStartOfDay()) &&
            employee.undesiredDates.includes(shiftEndDateString))) {
        return UNDESIRED_COLOR
    } else if (employee.desiredDates.includes(shiftStartDateString) ||
        // The contains() check is ignored for a shift end at midnight (00:00:00).
        (shiftEnd.isAfter(shiftStart.toLocalDate().plusDays(1).atStartOfDay()) &&
            employee.desiredDates.includes(shiftEndDateString))) {
        return DESIRED_COLOR
    } else {
        return " #729fcf"; // Tango Sky Blue
    }
}

function renderSchedule(schedule) {
    console.log('Rendering schedule:', schedule);

    if (!schedule) {
        console.error('No schedule data provided to renderSchedule');
        return;
    }

    refreshSolvingButtons(schedule.solverStatus != null && schedule.solverStatus !== "NOT_SOLVING");
    $("#score").text("Score: " + (schedule.score == null ? "?" : schedule.score));

    const unassignedShifts = $("#unassignedShifts");
    const groups = [];

    // Check if schedule.shifts exists and is an array
    if (!schedule.shifts || !Array.isArray(schedule.shifts) || schedule.shifts.length === 0) {
        console.warn('No shifts data available in schedule');
        return;
    }

    // Show only first 7 days of draft
    const scheduleStart = schedule.shifts.map(shift => JSJoda.LocalDateTime.parse(shift.start).toLocalDate()).sort()[0].toString();
    const scheduleEnd = JSJoda.LocalDate.parse(scheduleStart).plusDays(7).toString();

    windowStart = scheduleStart;
    windowEnd = scheduleEnd;

    unassignedShifts.children().remove();
    let unassignedShiftsCount = 0;
    byEmployeeGroupDataSet.clear();
    byLocationGroupDataSet.clear();

    byEmployeeItemDataSet.clear();
    byLocationItemDataSet.clear();

    // Check if schedule.employees exists and is an array
    if (!schedule.employees || !Array.isArray(schedule.employees)) {
        console.warn('No employees data available in schedule');
        return;
    }

    schedule.employees.forEach((employee, index) => {
        const employeeGroupElement = $('<div class="card-body p-2"/>')
            .append($(`<h5 class="card-title mb-2"/>)`)
                .append(employee.name))
            .append($('<div/>')
                .append($(employee.skills.map(skill => `<span class="badge me-1 mt-1" style="background-color:#d3d7cf">${skill}</span>`).join(''))));
        byEmployeeGroupDataSet.add({id: employee.name, content: employeeGroupElement.html()});

        employee.unavailableDates.forEach((rawDate, dateIndex) => {
            const date = JSJoda.LocalDate.parse(rawDate)
            const start = date.atStartOfDay().toString();
            const end = date.plusDays(1).atStartOfDay().toString();
            const byEmployeeShiftElement = $(`<div/>`)
                .append($(`<h5 class="card-title mb-1"/>`).text("Unavailable"));
            byEmployeeItemDataSet.add({
                id: "employee-" + index + "-unavailability-" + dateIndex, group: employee.name,
                content: byEmployeeShiftElement.html(),
                start: start, end: end,
                type: "background",
                style: "opacity: 0.5; background-color: " + UNAVAILABLE_COLOR,
            });
        });
        employee.undesiredDates.forEach((rawDate, dateIndex) => {
            const date = JSJoda.LocalDate.parse(rawDate)
            const start = date.atStartOfDay().toString();
            const end = date.plusDays(1).atStartOfDay().toString();
            const byEmployeeShiftElement = $(`<div/>`)
                .append($(`<h5 class="card-title mb-1"/>`).text("Undesired"));
            byEmployeeItemDataSet.add({
                id: "employee-" + index + "-undesired-" + dateIndex, group: employee.name,
                content: byEmployeeShiftElement.html(),
                start: start, end: end,
                type: "background",
                style: "opacity: 0.5; background-color: " + UNDESIRED_COLOR,
            });
        });
        employee.desiredDates.forEach((rawDate, dateIndex) => {
            const date = JSJoda.LocalDate.parse(rawDate)
            const start = date.atStartOfDay().toString();
            const end = date.plusDays(1).atStartOfDay().toString();
            const byEmployeeShiftElement = $(`<div/>`)
                .append($(`<h5 class="card-title mb-1"/>`).text("Desired"));
            byEmployeeItemDataSet.add({
                id: "employee-" + index + "-desired-" + dateIndex, group: employee.name,
                content: byEmployeeShiftElement.html(),
                start: start, end: end,
                type: "background",
                style: "opacity: 0.5; background-color: " + DESIRED_COLOR,
            });
        });
    });

    schedule.shifts.forEach((shift, index) => {
        if (groups.indexOf(shift.location) === -1) {
            groups.push(shift.location);
            byLocationGroupDataSet.add({
                id: shift.location,
                content: shift.location,
            });
        }

        if (shift.employee == null) {
            unassignedShiftsCount++;

            const byLocationShiftElement = $('<div class="card-body p-2"/>')
                .append($(`<h5 class="card-title mb-2"/>)`)
                    .append("Unassigned"))
                .append($('<div/>')
                    .append($(`<span class="badge me-1 mt-1" style="background-color:#d3d7cf">${shift.requiredSkill}</span>`)));

            byLocationItemDataSet.add({
                id: 'shift-' + index, group: shift.location,
                content: byLocationShiftElement.html(),
                start: shift.start, end: shift.end,
                style: "background-color: #EF292999"
            });
        } else {
            const skillColor = (shift.employee.skills.indexOf(shift.requiredSkill) === -1 ? '#ef2929' : '#8ae234');
            const byEmployeeShiftElement = $('<div class="card-body p-2"/>')
                .append($(`<h5 class="card-title mb-2"/>)`)
                    .append(shift.location))
                .append($('<div/>')
                    .append($(`<span class="badge me-1 mt-1" style="background-color:${skillColor}">${shift.requiredSkill}</span>`)));
            const byLocationShiftElement = $('<div class="card-body p-2"/>')
                .append($(`<h5 class="card-title mb-2"/>)`)
                    .append(shift.employee.name))
                .append($('<div/>')
                    .append($(`<span class="badge me-1 mt-1" style="background-color:${skillColor}">${shift.requiredSkill}</span>`)));

            const shiftColor = getShiftColor(shift, shift.employee);
            byEmployeeItemDataSet.add({
                id: 'shift-' + index, group: shift.employee.name,
                content: byEmployeeShiftElement.html(),
                start: shift.start, end: shift.end,
                style: "background-color: " + shiftColor
            });
            byLocationItemDataSet.add({
                id: 'shift-' + index, group: shift.location,
                content: byLocationShiftElement.html(),
                start: shift.start, end: shift.end,
                style: "background-color: " + shiftColor
            });
        }
    });

    if (unassignedShiftsCount === 0) {
        unassignedShifts.append($(`<p/>`).text(`There are no unassigned shifts.`));
    } else {
        unassignedShifts.append($(`<p/>`).text(`There are ${unassignedShiftsCount} unassigned shifts.`));
    }
    byEmployeeTimeline.setWindow(scheduleStart, scheduleEnd);
    byLocationTimeline.setWindow(scheduleStart, scheduleEnd);
}
